/// Represents the role of an agent in the system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRole {
    /// Document Coordinator - orchestrates the documentation process
    DocCoordinator,
    
    /// Document Content Syncer - analyzes and synchronizes document content
    DocContentSyncer,
    
    /// Docusaurus Expert - manages Docusaurus-specific configurations
    DocusaurusExpert,
    
    /// UI Design Expert - optimizes documentation UI elements
    UiDesignExpert,
    
    /// CSS Expert - enhances documentation styling
    CssExpert,
    
    /// Document Runner - executes documentation operations
    DocRunner,
    
    /// Document Verifier - validates documentation results
    DocVerifier,
    
    /// Fix Planner - diagnoses issues and plans fixes
    FixPlanner,
    
    /// Implementer - implements planned changes
    Implementer,
    
    /// Refactoring Guru - plans code refactoring
    RefactoringGuru,
    
    /// Code Reviewer - reviews code for quality and issues
    CodeReviewer,
    
    /// Git Committer - handles git commit operations
    GitCommitter,
    
    /// Custom role with a specified name
    Custom(String),
}

impl AgentRole {
    /// Get the name of the role
    pub fn name(&self) -> String {
        match self {
            Self::DocCoordinator => "DocCoordinator".to_string(),
            Self::DocContentSyncer => "DocContentSyncer".to_string(),
            Self::DocusaurusExpert => "DocusaurusExpert".to_string(),
            Self::UiDesignExpert => "UiDesignExpert".to_string(),
            Self::CssExpert => "CssExpert".to_string(),
            Self::DocRunner => "DocRunner".to_string(),
            Self::DocVerifier => "DocVerifier".to_string(),
            Self::FixPlanner => "FixPlanner".to_string(),
            Self::Implementer => "Implementer".to_string(),
            Self::RefactoringGuru => "RefactoringGuru".to_string(),
            Self::CodeReviewer => "CodeReviewer".to_string(),
            Self::GitCommitter => "GitCommitter".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
    
    /// Create an AgentRole from a string
    pub fn from_str(role_name: &str) -> Self {
        match role_name {
            "DocCoordinator" => Self::DocCoordinator,
            "DocContentSyncer" => Self::DocContentSyncer,
            "DocusaurusExpert" => Self::DocusaurusExpert,
            "UiDesignExpert" => Self::UiDesignExpert,
            "CssExpert" => Self::CssExpert,
            "DocRunner" => Self::DocRunner,
            "DocVerifier" => Self::DocVerifier,
            "FixPlanner" => Self::FixPlanner,
            "Implementer" => Self::Implementer,
            "RefactoringGuru" => Self::RefactoringGuru,
            "CodeReviewer" => Self::CodeReviewer,
            "GitCommitter" => Self::GitCommitter,
            _ => Self::Custom(role_name.to_string()),
        }
    }
} 