use std::fs;
use std::time::Instant;

pub struct Timer {
    stages: Vec<(String, f64)>,
    start: Instant,
    current_stage: String,
}

impl Timer {
    pub fn new() -> Self {
        fs::create_dir_all("../comparacao_tempos").ok();
        Timer {
            stages: Vec::new(),
            start: Instant::now(),
            current_stage: String::new(),
        }
    }

    pub fn begin(&mut self, name: &str) {
        self.current_stage = name.to_string();
        self.start = Instant::now();
    }

    pub fn end(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.stages.push((self.current_stage.clone(), elapsed));
        println!("   ⏱  {}: {:.4}s", self.current_stage, elapsed);
    }

    pub fn save_csv(&self) {
        let mut csv = String::from("projeto,etapa,tempo_segundos\n");
        for (stage, secs) in &self.stages {
            csv.push_str(&format!("Rust,{},{:.4}\n", stage, secs));
        }
        let total: f64 = self.stages.iter().map(|(_, s)| s).sum();
        csv.push_str(&format!("Rust,total,{:.4}\n", total));
        fs::write("../comparacao_tempos/tempos_rust.csv", csv).ok();
        println!("\n   ⏱  Tempos salvos em comparacao_tempos/tempos_rust.csv");
        println!("   ⏱  Tempo TOTAL: {:.4}s\n", total);
    }
}
