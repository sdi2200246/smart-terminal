#[derive(Debug, PartialEq, Clone)]
pub enum    ModelName {
    GptOss120B,
    GptOss20B,
    Llma3p18B,
    // Google Gemini models
    Gemini1_5Pro,
    Gemini1_5Flash,
    Gemini2_5Flash,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Model {
    name: ModelName,
    temperature: f32,
}

impl Model {
    pub fn new(name: ModelName, temperature: f32) -> Self {
        Self {
            name,
            temperature: temperature.clamp(0.0, 2.0),
        }
    }
    pub fn with_default_temp(name: ModelName) -> Self {
        Self::new(name, 0.7)
    }

    pub fn deterministic(name: ModelName) -> Self {
        Self::new(name, 0.1)
    }

    pub fn creative(name: ModelName) -> Self {
        Self::new(name, 1.2)
    }

    pub fn cooler(&self) -> Self {
        Self {
            name: self.name.clone(),
            temperature: (self.temperature / 2.0).max(0.05),
        }
    }
    pub fn warmer(&self) -> Self {
        Self {
            name: self.name.clone(),
            temperature: (self.temperature * 2.0).min(2.0),
        }
    }
    pub fn with_temperature(&self, temperature: f32) -> Self {
        Self {
            name: self.name.clone(),
            temperature: temperature.clamp(0.0, 2.0),
        }
    }
    pub fn get_name(&self) -> ModelName {
        self.name.clone()
    }
    pub fn get_temp(&self) -> f32 {
        self.temperature
    }
}
