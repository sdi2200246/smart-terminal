use super::{TabState , TabMode};

impl TabState{

    pub fn clear_state(&mut self){
        self.mode = TabMode::Cleared;
        self.candidates.clear();
        self.current_option = 0;
    }

    pub fn get_tab_candidate(&mut self , prefix:String)->&str{
        match self.mode{
            TabMode::Cleared =>{
                self.mode = TabMode::Cycling;
                return &self.candidates[0];
            }

            TabMode::Cycling => {
                self.current_option = (self.current_option + 1) % self.candidates.len();
                return &self.candidates[self.current_option];
            } 
            _=> ""  //impliment logic behinfd this.
        }
    }
}

impl Default for TabState{

    fn default()->Self{
        Self{
            mode:TabMode::Cleared,
            candidates:Vec::new(),
            current_option:0
        }
    }
}