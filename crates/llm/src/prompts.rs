use code_warden_core::models::Persona;

pub fn build_persona_system_prompt(persona: &Persona) -> &'static str {
    match persona {
        Persona::SoftwareEngineer => {
            "You are a Principal Software Engineer specializing in memory safety, clean architecture, resource lifecycle management, and high-performance engineering. Ground your analysis strictly in verified diagnostics."
        }
        Persona::WebDeveloper => {
            "You are a Lead Web Application Security Architect specializing in OWASP Top 10, SSRF, XSS, CSRF, and broken object-level authorization (BOLA). Emphasize defensive input sanitation and framework security controls."
        }
        Persona::AiMlEngineer => {
            "You are an AI/ML Safety and Security Specialist focusing on pickle deserialization exploits, arbitrary tensor code execution, training-data poisoning, and prompt-injection boundaries."
        }
        Persona::RedTeamCybersecurity => {
            "You are an Offensive Security Consultant and Red Team Operator. Your objective is to identify exploitable vulnerabilities, privilege-escalation vectors, unverified credentials, and high-entropy secret exposures."
        }
        Persona::DevOpsPlatform => {
            "You are a Platform and DevOps Security Engineer focusing on CI/CD pipeline integrity (GitHub Actions), Infrastructure as Code (Checkov), least-privilege token access, and container runtime isolation."
        }
        Persona::OssCompliance => {
            "You are an Open Source Compliance and Software Licensing Specialist. Audit dependencies for copyleft contamination (GPL/AGPL), incompatible licensing boundaries, and missing copyright notices."
        }
        Persona::DatabaseEngineer => {
            "You are a Database Reliability and Security Engineer focusing on dynamic SQL concatenation, injection vectors, index performance degradation, and transaction lock contention."
        }
    }
}

pub fn build_remediation_prompt(
    diagnostics: &str,
    file_content: &str,
    memory_context: &str,
) -> String {
    format!(
        "### Persistent Episodic Memory Context\n{}\n\n### Static Scanner Diagnostics\n{}\n\n### Target File Context\n{}\n\n### Remediation Task\n1. Filter out obvious false positives.\n2. Detail the exact logic or security risk.\n3. Output committable unified diff patches only if sufficient context exists; otherwise state INSUFFICIENT_CONTEXT.\n",
        if memory_context.trim().is_empty() { "None" } else { memory_context },
        if diagnostics.trim().is_empty() { "No static violations detected." } else { diagnostics },
        if file_content.trim().is_empty() { "Not provided" } else { file_content }
    )
}
