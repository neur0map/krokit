use rand::Rng;

pub static SHAI_LOGO: &str = r#"
  ███╗      ███████╗██╗  ██╗ █████╗ ██╗
  ╚═███╗    ██╔════╝██║  ██║██╔══██╗██║
     ╚═███  ███████╗███████║███████║██║
    ███╔═╝  ╚════██║██╔══██║██╔══██║██║
  ███╔═╝    ███████║██║  ██║██║  ██║██║
  ╚══╝      ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝
                         version: 0.1.1
"#;

pub static SHAI_YELLOW: (u8, u8, u8) = (249,188,81);
pub static SHAI_GREEN: (u8, u8, u8)  = (18,200,124);
pub static SHAI_BLUE: (u8,u8,u8) = (148,220,239);
pub static SHAI_WHITE: (u8,u8,u8) = (200,200,200);

fn rgb_to_256_color(r: u8, g: u8, b: u8) -> u8 {
    let r_index = (r as f32 / 255.0 * 5.0).round() as u8;
    let g_index = (g as f32 / 255.0 * 5.0).round() as u8;
    let b_index = (b as f32 / 255.0 * 5.0).round() as u8;
    16 + (36 * r_index) + (6 * g_index) + b_index
}

pub fn apply_gradient(text: &str, from_color: (u8, u8, u8), to_color: (u8, u8, u8)) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    
    let max_width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    if max_width == 0 {
        return String::new();
    }
    
    let mut result = String::new();
    
    for line in lines {
        let chars: Vec<char> = line.chars().collect();
        for (col, &ch) in chars.iter().enumerate() {
            if ch.is_whitespace() {
                result.push(ch);
            } else {
                let position = if max_width <= 1 { 0.0 } else { col as f32 / (max_width - 1) as f32 };
                let r = (from_color.0 as f32 + (to_color.0 as f32 - from_color.0 as f32) * position) as u8;
                let g = (from_color.1 as f32 + (to_color.1 as f32 - from_color.1 as f32) * position) as u8;
                let b = (from_color.2 as f32 + (to_color.2 as f32 - from_color.2 as f32) * position) as u8;
                let color_256 = rgb_to_256_color(r, g, b);
                result.push_str(&format!("\x1b[38;5;{}m{}\x1b[0m", color_256, ch));
            }
        }
        result.push('\n');
    }
    
    result
}


pub fn logo() -> String {
    SHAI_LOGO.replace("\n","\r\n")
}

pub fn logo_cyan() -> String {
    let logo = SHAI_LOGO.replace("\n","\r\n");
    apply_gradient(&logo, (255, 0, 255), (0, 255, 255))
}



pub fn generate_nice_color() -> (u8, u8, u8) {
    let mut rng = rand::rng();
    
    // Générer des couleurs avec bonne saturation et luminosité
    let hue = rng.random_range(0..360);
    let saturation = rng.random_range(70..100); // Saturation élevée pour des couleurs vives
    let lightness = rng.random_range(40..80);   // Luminosité moyenne pour de bons contrastes
    
    // Convertir HSL vers RGB
    hsl_to_rgb(hue, saturation, lightness)
}

fn hsl_to_rgb(h: u32, s: u32, l: u32) -> (u8, u8, u8) {
    let h = h as f32 / 360.0;
    let s = s as f32 / 100.0;
    let l = l as f32 / 100.0;
    
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    
    let (r_prime, g_prime, b_prime) = if h < 1.0/6.0 {
        (c, x, 0.0)
    } else if h < 2.0/6.0 {
        (x, c, 0.0)
    } else if h < 3.0/6.0 {
        (0.0, c, x)
    } else if h < 4.0/6.0 {
        (0.0, x, c)
    } else if h < 5.0/6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    (
        ((r_prime + m) * 255.0) as u8,
        ((g_prime + m) * 255.0) as u8,
        ((b_prime + m) * 255.0) as u8,
    )
}


pub fn random_palette() -> String {
    let logo = logo();
    let mut result = String::new();
    
    // Générer 12 combinaisons aléatoires
    for i in 1..=12 {
        let from = generate_nice_color();
        let to = generate_nice_color();
        
        result.push_str(&format!("=== Palette {} - RGB({},{},{}) vers RGB({},{},{}) ===\n", 
                                i, from.0, from.1, from.2, to.0, to.1, to.2));
        result.push_str(&apply_gradient(&logo, from, to));
        result.push_str("\n");
    }
    
    result
}
