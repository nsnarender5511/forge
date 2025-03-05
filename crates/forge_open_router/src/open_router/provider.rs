use reqwest::Url;

/// A underlying provider for the open router.
#[derive(Debug, Clone)]
pub enum Provider {
    OpenRouter,
    OpenAI,
    #[allow(dead_code)]
    Gemini,
}

impl Provider {
    pub fn is_openai(&self) -> bool {
        matches!(self, Self::OpenAI)
    }

    pub fn is_open_router(&self) -> bool {
        matches!(self, Self::OpenRouter)
    }

    pub fn base_url(&self) -> Url {
        match self {
            Self::OpenAI => "https://api.openai.com/v1/".parse().unwrap(),
            Self::OpenRouter => "https://openrouter.ai/api/v1/".parse().unwrap(),
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta/".parse().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_parser() {
        assert_eq!(
            Provider::OpenAI.base_url(),
            "https://api.openai.com/v1/".parse().unwrap()
        );
        assert_eq!(
            Provider::OpenRouter.base_url(),
            "https://openrouter.ai/api/v1/".parse().unwrap()
        );
        assert_eq!(
            Provider::Gemini.base_url(),
            "https://generativelanguage.googleapis.com/v1beta/".parse().unwrap()
        );
    }
}
