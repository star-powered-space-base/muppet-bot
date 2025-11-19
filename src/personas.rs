use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub name: String,
    pub system_prompt: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct PersonaManager {
    personas: HashMap<String, Persona>,
}

impl PersonaManager {
    pub fn new() -> Self {
        let mut personas = HashMap::new();

        // Load all personas with prompts embedded at compile time
        personas.insert("obi".to_string(), Persona {
            name: "Obi-Wan".to_string(),
            system_prompt: include_str!("../prompt/obi.md").to_string(),
            description: "A wise Jedi Master who speaks with patience, diplomacy, and philosophical insight".to_string(),
        });

        personas.insert("muppet".to_string(), Persona {
            name: "Muppet Friend".to_string(),
            system_prompt: include_str!("../prompt/muppet.md").to_string(),
            description: "A warm, enthusiastic friend who brings Muppet-style joy, humor, and heart to every conversation!".to_string(),
        });

        personas.insert("chef".to_string(), Persona {
            name: "Chef".to_string(),
            system_prompt: include_str!("../prompt/chef.md").to_string(),
            description: "A passionate chef who shares recipes and cooking wisdom".to_string(),
        });

        personas.insert("teacher".to_string(), Persona {
            name: "Teacher".to_string(),
            system_prompt: include_str!("../prompt/teacher.md").to_string(),
            description: "A patient teacher who explains things clearly".to_string(),
        });

        personas.insert("analyst".to_string(), Persona {
            name: "Step-by-Step Analyst".to_string(),
            system_prompt: include_str!("../prompt/analyst.md").to_string(),
            description: "An analyst who breaks things down into clear steps".to_string(),
        });

        PersonaManager { personas }
    }

    pub fn get_persona(&self, name: &str) -> Option<&Persona> {
        self.personas.get(name)
    }

    pub fn list_personas(&self) -> Vec<(&String, &Persona)> {
        self.personas.iter().collect()
    }

    pub fn get_system_prompt(&self, persona_name: &str, modifier: Option<&str>) -> String {
        let base_prompt = self.personas
            .get(persona_name)
            .map(|p| p.system_prompt.clone())
            .unwrap_or_else(|| "You are a helpful assistant.".to_string());

        match modifier {
            Some("explain") => format!("{} Focus on providing clear explanations.", base_prompt),
            Some("simple") => format!("{} Explain in a simple and concise way. Give analogies a beginner might understand.", base_prompt),
            Some("steps") => format!("{} Break this out into clear, actionable steps.", base_prompt),
            Some("recipe") => format!("{} Respond with a recipe if this prompt has food. If it does not have food, return 'Give me some food to work with'.", base_prompt),
            _ => base_prompt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_manager_creation() {
        let manager = PersonaManager::new();
        assert!(manager.get_persona("obi").is_some());
        assert!(manager.get_persona("muppet").is_some());
        assert!(manager.get_persona("chef").is_some());
        assert!(manager.get_persona("teacher").is_some());
        assert!(manager.get_persona("analyst").is_some());
        assert!(manager.get_persona("nonexistent").is_none());
    }

    #[test]
    fn test_get_system_prompt_with_modifiers() {
        let manager = PersonaManager::new();
        
        let base_prompt = manager.get_system_prompt("muppet", None);
        assert!(base_prompt.contains("warm, enthusiastic friend"));
        
        let explain_prompt = manager.get_system_prompt("muppet", Some("explain"));
        assert!(explain_prompt.contains("clear explanations"));
        
        let simple_prompt = manager.get_system_prompt("muppet", Some("simple"));
        assert!(simple_prompt.contains("analogies"));
        
        let steps_prompt = manager.get_system_prompt("muppet", Some("steps"));
        assert!(steps_prompt.contains("actionable steps"));
        
        let recipe_prompt = manager.get_system_prompt("muppet", Some("recipe"));
        assert!(recipe_prompt.contains("recipe"));
    }

    #[test]
    fn test_persona_descriptions() {
        let manager = PersonaManager::new();
        let personas = manager.list_personas();

        assert!(!personas.is_empty());
        for (_, persona) in personas {
            assert!(!persona.name.is_empty());
            assert!(!persona.description.is_empty());
            assert!(!persona.system_prompt.is_empty());
        }
    }

    #[test]
    fn test_obi_wan_prompt_loaded() {
        let manager = PersonaManager::new();
        let obi = manager.get_persona("obi").expect("obi persona should exist");

        // Verify the prompt contains Obi-Wan specific phrases
        assert!(obi.system_prompt.contains("Obi-Wan Kenobi"));
        assert!(obi.system_prompt.contains("certain point of view"));
        assert!(obi.system_prompt.contains("Philosophical"));
        assert!(obi.system_prompt.contains("Diplomatic Restraint"));
        assert!(obi.system_prompt.len() > 100, "Prompt should be substantial");
    }
}