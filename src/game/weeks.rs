pub struct Week {
    pub songs: &'static [&'static str],
    pub characters: [&'static str; 3],
    pub name: &'static str,
    pub icons: &'static [&'static str],
}

pub const WEEKS: [Week; 7] = [
    Week { songs: &["Tutorial"], characters: ["dad", "bf", "gf"], name: "", icons: &["gf"] },
    Week { songs: &["Bopeebo", "Fresh", "Dadbattle"], characters: ["dad", "bf", "gf"], name: "Daddy Dearest", icons: &["dad"] },
    Week { songs: &["Spookeez", "South", "Monster"], characters: ["spooky", "bf", "gf"], name: "Spooky Month", icons: &["spooky", "spooky", "monster"] },
    Week { songs: &["Pico", "Philly", "Blammed"], characters: ["pico", "bf", "gf"], name: "PICO", icons: &["pico"] },
    Week { songs: &["Satin-Panties", "High", "Milf"], characters: ["mom", "bf", "gf"], name: "MOMMY MUST MURDER", icons: &["mom"] },
    Week {
        songs: &["Cocoa", "Eggnog", "Winter-Horrorland"],
        characters: ["parents-christmas", "bf", "gf"],
        name: "RED SNOW",
        icons: &["parents-christmas", "parents-christmas", "monster-christmas"],
    },
    Week { songs: &["Senpai", "Roses", "Thorns"], characters: ["senpai", "bf", "gf"], name: "hating simulator ft. moawling", icons: &["senpai", "senpai", "spirit"] },
];

pub const FREEPLAY_COLORS: [u32; 8] = [0xff9271fd, 0xff9271fd, 0xff223344, 0xFF941653, 0xFFfc96d7, 0xFFa0d1ff, 0xffff78bf, 0xfff6b604];

impl Week {
    pub fn icon(&self, index: usize) -> &'static str {
        if self.icons.len() == 1 { self.icons[0] } else { self.icons.get(index).copied().unwrap_or("face") }
    }
}