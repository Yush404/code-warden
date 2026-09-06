pub mod memory;
pub mod prompts;

use anyhow::{bail, Context, Result};
use code_warden_core::models::Persona;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Ollama {
        endpoint: String,
        model: String,
    },
    Gemini {
        api_key: String,
        model: String,
    },
    Hybrid {
        api_key: String,
        gemini_model: String,
        ollama_endpoint: String,
        ollama_model: String,
    },
}

pub struct LlmGateway {
    client: reqwest::Client,
}

impl Default for LlmGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmGateway {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn resolve_dynamic_gemini_model(&self, api_key: &str) -> String {
        let list_url = "https://generativelanguage.googleapis.com/v1beta/models";
        let res = self
            .client
            .get(list_url)
            .header("x-goog-api-key", api_key)
            .send()
            .await;

        if let Ok(response) = res {
            if response.status().is_success() {
                if let Ok(body) = response.json::<Value>().await {
                    if let Some(models) = body["models"].as_array() {
                        let mut candidates: Vec<String> = models
                            .iter()
                            .filter_map(|m| {
                                let name = m["name"]
                                    .as_str()?
                                    .trim_start_matches("models/")
                                    .to_string();
                                let methods = m["supportedGenerationMethods"].as_array()?;
                                let supports_generate = methods
                                    .iter()
                                    .any(|method| method.as_str() == Some("generateContent"));

                                if supports_generate
                                    && !name.contains("embedding")
                                    && !name.contains("aqa")
                                    && !name.contains("imagen")
                                {
                                    Some(name)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        candidates.sort_by_key(|name| {
                            if name.contains("flash-lite") {
                                1
                            } else if name.contains("flash") {
                                2
                            } else if name.contains("pro") {
                                3
                            } else {
                                4
                            }
                        });

                        if let Some(best) = candidates.first() {
                            return best.clone();
                        }
                    }
                }
            }
        }
        "gemini-2.5-flash".to_string()
    }

    async fn execute_ollama(
        &self,
        endpoint: &str,
        model: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));
        let body = json!({
            "model": model,
            "stream": false,
            "options": { "temperature": 0.1 },
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ]
        });

        let res = self.client.post(&url).json(&body).send().await.context(
            "Failed to connect to local Ollama daemon. Ensure 'ollama serve' is running.",
        )?;

        if !res.status().is_success() {
            let status = res.status();
            let err = res.text().await.unwrap_or_default();
            bail!("Ollama error ({}): {}", status, err);
        }

        let val: Value = res.json().await?;
        Ok(val["message"]["content"]
            .as_str()
            .unwrap_or("No response content")
            .to_string())
    }

    async fn execute_gemini(
        &self,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String> {
        if api_key.trim().is_empty() {
            bail!("Gemini API key is empty.");
        }

        let resolved_model = if model.is_empty()
            || model == "auto"
            || model == "gemini-2.5-flash"
            || model == "gemini-3.6-flash"
        {
            let discovered = self.resolve_dynamic_gemini_model(api_key).await;
            println!(
                "\x1b[36m[*] Auto-selected Gemini model:\x1b[0m {}",
                discovered
            );
            discovered
        } else {
            model.to_string()
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            resolved_model
        );

        let body = json!({
            "systemInstruction": {
                "parts": [{ "text": system_prompt }]
            },
            "contents": [{
                "role": "user",
                "parts": [{ "text": user_prompt }]
            }],
            "generationConfig": {
                "temperature": 0.1
            }
        });

        let mut attempts = 0;
        let max_attempts = 2;
        let mut last_err = String::new();

        while attempts < max_attempts {
            attempts += 1;
            let res = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("x-goog-api-key", api_key)
                .json(&body)
                .send()
                .await
                .context("Failed to dispatch request to Google Gemini API")?;

            let status = res.status();
            if status.is_success() {
                let val: Value = res.json().await?;
                let text = val["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("No content returned from Gemini.")
                    .to_string();
                return Ok(text);
            }

            let err_text = res.text().await.unwrap_or_default();
            if status.as_u16() == 503 || status.as_u16() == 429 {
                last_err = format!(
                    "(Attempt {}/{}) API Busy ({}): {}",
                    attempts, max_attempts, status, err_text
                );
                std::thread::sleep(Duration::from_secs(2 * attempts as u64));
                continue;
            }

            bail!("Gemini API Error ({}): {}", status, err_text);
        }

        bail!(
            "Gemini API Error after {} retries: {}",
            max_attempts,
            last_err
        );
    }

    pub async fn review_and_patch(
        &self,
        provider: &Provider,
        persona: &Persona,
        diagnostics: &str,
        file_content: &str,
        memory_context: &str,
    ) -> Result<String> {
        let system_prompt = prompts::build_persona_system_prompt(persona);
        let user_prompt =
            prompts::build_remediation_prompt(diagnostics, file_content, memory_context);

        match provider {
            Provider::Ollama { endpoint, model } => {
                println!("\x1b[35m[*] Routing to local Ollama ({})\x1b[0m", model);
                self.execute_ollama(endpoint, model, &system_prompt, &user_prompt)
                    .await
            }

            Provider::Gemini { api_key, model } => {
                self.execute_gemini(api_key, model, &system_prompt, &user_prompt)
                    .await
            }

            Provider::Hybrid {
                api_key,
                gemini_model,
                ollama_endpoint,
                ollama_model,
            } => {
                if !api_key.trim().is_empty() {
                    match self
                        .execute_gemini(api_key, gemini_model, &system_prompt, &user_prompt)
                        .await
                    {
                        Ok(resp) => return Ok(resp),
                        Err(e) => {
                            println!(
                                "\x1b[33m[!] Gemini unavailable or failed ({}). Falling back to local Ollama ({}) ...\x1b[0m",
                                e, ollama_model
                            );
                        }
                    }
                } else {
                    println!("\x1b[33m[*] No Gemini API key detected. Defaulting to local Ollama ({}) ...\x1b[0m", ollama_model);
                }

                self.execute_ollama(ollama_endpoint, ollama_model, &system_prompt, &user_prompt)
                    .await
            }
        }
    }
}
