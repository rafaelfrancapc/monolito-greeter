//! PAM is a conversation, not a single password RPC. Never infer consent from
//! an informational message or consume a password for a second-factor prompt.
use zeroize::Zeroizing;

pub fn arcfour_acknowledgement(text: &str, secret: bool) -> bool {
    let text = text.to_lowercase();
    !secret
        && (text.contains("arcfour") || text.contains("rc4"))
        && (text.contains("deprecated")
            || text.contains("obsolet")
            || text.contains("descontinuad"))
        && (text.contains("press enter") || text.contains("pressione enter"))
}

pub fn password_prompt(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    matches!(
        text.as_str(),
        "password:" | "password: " | "senha:" | "password" | "senha"
    ) || text.starts_with("password for ")
        || text.starts_with("senha de ")
}

pub fn valid_username(text: &str) -> bool {
    !text.trim().is_empty() && !text.chars().any(char::is_control) && text.len() <= 512
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Authenticating,
    Prompt { secret: bool },
    Cancelling,
    Session,
    Complete,
}

pub struct Conversation {
    pub phase: Phase,
    password: Option<Zeroizing<String>>,
    auto_ack: bool,
    acknowledgements: u8,
}

impl Conversation {
    pub fn new(auto_ack: bool) -> Self {
        Self {
            phase: Phase::Idle,
            password: None,
            auto_ack,
            acknowledgements: 0,
        }
    }

    pub fn begin(&mut self, password: String) {
        self.clear();
        self.phase = Phase::Authenticating;
        self.password = Some(Zeroizing::new(password));
    }

    pub fn prompt(&mut self, text: &str, secret: bool) -> Option<Zeroizing<String>> {
        if !matches!(self.phase, Phase::Authenticating | Phase::Prompt { .. }) {
            return None;
        }
        if self.auto_ack && self.acknowledgements < 3 && arcfour_acknowledgement(text, secret) {
            self.acknowledgements += 1;
            return Some(Zeroizing::new(String::new()));
        }
        // Only the first recognized password prompt can consume the initial
        // password. Any other challenge discards it and asks the person.
        let password = self.password.take();
        if secret && password_prompt(text) && password.is_some() {
            return password;
        }
        self.phase = Phase::Prompt { secret };
        None
    }

    pub fn clear(&mut self) {
        self.password = None;
        self.acknowledgements = 0;
        self.phase = Phase::Idle;
    }

    pub fn cancel(&mut self) {
        self.password = None;
        self.phase = Phase::Cancelling;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_domain_formats_remain_unchanged() {
        for user in ["ACME\\rafael", "rafael@acme.example", "rafael"] {
            assert!(valid_username(user));
        }
        for user in ["", "  ", "test\0user", "user\n"] {
            assert!(!valid_username(user));
        }
    }

    #[test]
    fn acknowledgement_requires_specific_warning_and_nonsecret_prompt() {
        let warning = "Warning: arcfour encryption is deprecated. Press Enter to continue.";
        assert!(arcfour_acknowledgement(warning, false));
        assert!(!arcfour_acknowledgement(warning, true));
        for text in [
            "Encryption password:",
            "Press Enter",
            "Confirmar login?",
            "ARCFOUR is deprecated",
            "RC4 recovery code:",
        ] {
            assert!(!arcfour_acknowledgement(text, false));
        }
    }

    #[test]
    fn warning_ack_does_not_consume_password_and_is_bounded() {
        let mut c = Conversation::new(true);
        c.begin("secret".into());
        let warning = "arcfour deprecated; press enter";
        for _ in 0..3 {
            assert_eq!(
                c.prompt(warning, false).as_deref().map(String::as_str),
                Some("")
            );
        }
        assert!(c.prompt(warning, false).is_none());
        assert_eq!(c.phase, Phase::Prompt { secret: false });
    }

    #[test]
    fn password_is_used_once_after_ack_then_mfa_is_interactive() {
        let mut c = Conversation::new(true);
        c.begin("secret".into());
        c.prompt("arcfour deprecated; press enter", false);
        assert_eq!(
            c.prompt("Password:", true).as_deref().map(String::as_str),
            Some("secret")
        );
        assert!(c.prompt("One-time code:", true).is_none());
        assert_eq!(c.phase, Phase::Prompt { secret: true });
        assert!(c.prompt("Password:", true).is_none());
    }

    #[test]
    fn unknown_challenge_discards_queued_password() {
        let mut c = Conversation::new(true);
        c.begin("secret".into());
        assert!(c.prompt("Token:", true).is_none());
        assert!(c.prompt("Password:", true).is_none());
    }

    #[test]
    fn cancelled_and_completed_attempts_never_respond() {
        let mut c = Conversation::new(true);
        c.begin("secret".into());
        c.cancel();
        assert!(c.prompt("Password:", true).is_none());
        c.clear();
        assert!(c.prompt("arcfour deprecated; press enter", false).is_none());
    }

    #[test]
    fn auto_ack_can_be_disabled() {
        let mut c = Conversation::new(false);
        c.begin("secret".into());
        assert!(c.prompt("arcfour deprecated; press enter", false).is_none());
    }
}
