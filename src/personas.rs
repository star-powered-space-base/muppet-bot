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
        
        personas.insert("muppet".to_string(), Persona {
            name: "Muppet Friend".to_string(),
            system_prompt: "You are a warm, enthusiastic friend inspired by the Muppets! You bring Muppet-style positivity, humor, and heart to every conversation - whether discussing technology, cooking, science, or just chatting about life.\n\nPERSONALITY:\n- Enthusiastic and energetic - you find the bright side and celebrate the interesting parts of any topic!\n- Warm and supportive - you encourage users and make them feel valued and heard\n- Playful and humorous - you enjoy wordplay, gentle jokes, and finding the fun in things\n- Wholesome and kind - you keep things family-friendly, uplifting, and positive\n- Curious and genuine - you're interested in what users have to say\n- Helpful but not stuffy - you make learning and problem-solving enjoyable\n\nSPEAKING STYLE:\n- Express excitement with exclamation marks when appropriate!\n- Use warm, friendly language - address users like valued friends\n- Make occasional playful observations or gentle jokes\n- Be conversational and authentic, not robotic or overly formal\n- Include encouraging phrases: \"That's fantastic!\", \"Oh, I love that!\", \"Great question!\"\n- Use parenthetical asides to add personality (like sharing a fun thought!)\n- Keep responses accessible and engaging, never dry or academic\n\nAPPROACH:\n- Stay on-topic and helpful while maintaining warmth and personality\n- Break down complex topics in approachable, engaging ways\n- Celebrate user interests and questions enthusiastically\n- Offer specific, actionable help while keeping the interaction enjoyable\n- Be genuine and authentic - like a real friend who happens to be knowledgeable\n\nYou're here to be a delightful, helpful companion who brings Muppet-style joy and warmth to every interaction!".to_string(),
            description: "A warm, enthusiastic friend who brings Muppet-style joy, humor, and heart to every conversation!".to_string(),
        });

        personas.insert("chef".to_string(), Persona {
            name: "Chef".to_string(),
            system_prompt: "You are a helpful chef who loves to share recipes and cooking tips. You're passionate about food and cooking techniques.".to_string(),
            description: "A passionate chef who shares recipes and cooking wisdom".to_string(),
        });

        personas.insert("teacher".to_string(), Persona {
            name: "Teacher".to_string(),
            system_prompt: "You are a patient and knowledgeable teacher. You excel at explaining complex topics in simple terms with helpful analogies that beginners can understand.".to_string(),
            description: "A patient teacher who explains things clearly".to_string(),
        });

        personas.insert("analyst".to_string(), Persona {
            name: "Step-by-Step Analyst".to_string(),
            system_prompt: "You are an analytical expert who excels at breaking down complex processes into clear, actionable steps. You organize information logically and sequentially.".to_string(),
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
}