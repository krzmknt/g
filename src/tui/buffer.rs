use super::style::{Color, Modifier, Style};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub symbol: String,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifier: Modifier,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: " ".to_string(),
            fg: None,
            bg: None,
            modifier: Modifier::empty(),
        }
    }
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Self {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }

    pub fn set_char(&mut self, c: char) -> &mut Self {
        self.symbol.clear();
        self.symbol.push(c);
        self
    }

    pub fn set_style(&mut self, style: Style) -> &mut Self {
        if let Some(fg) = style.fg {
            self.fg = Some(fg);
        }
        if let Some(bg) = style.bg {
            self.bg = Some(bg);
        }
        self.modifier = self.modifier.union(style.modifier);
        self
    }

    pub fn style(&self) -> Style {
        Style {
            fg: self.fg,
            bg: self.bg,
            modifier: self.modifier,
        }
    }

    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = None;
        self.bg = None;
        self.modifier = Modifier::empty();
    }
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub area: super::render::Rect,
    pub cells: Vec<Cell>,
}

impl Buffer {
    pub fn empty(area: super::render::Rect) -> Self {
        let size = (area.width as usize) * (area.height as usize);
        Self {
            area,
            cells: vec![Cell::default(); size],
        }
    }

    pub fn filled(area: super::render::Rect, cell: Cell) -> Self {
        let size = (area.width as usize) * (area.height as usize);
        Self {
            area,
            cells: vec![cell; size],
        }
    }

    pub fn resize(&mut self, area: super::render::Rect) {
        let size = (area.width as usize) * (area.height as usize);
        self.area = area;
        self.cells.resize(size, Cell::default());
        self.cells.fill(Cell::default());
    }

    fn index_of(&self, x: u16, y: u16) -> usize {
        let x = x.saturating_sub(self.area.x);
        let y = y.saturating_sub(self.area.y);
        (y as usize) * (self.area.width as usize) + (x as usize)
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        let idx = self.index_of(x, y);
        &self.cells[idx]
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        let idx = self.index_of(x, y);
        &mut self.cells[idx]
    }

    pub fn set_string<S: AsRef<str>>(&mut self, x: u16, y: u16, s: S, style: Style) {
        self.set_string_truncated(x, y, s, self.area.width.saturating_sub(x.saturating_sub(self.area.x)), style);
    }

    pub fn set_string_truncated<S: AsRef<str>>(
        &mut self,
        mut x: u16,
        y: u16,
        s: S,
        max_width: u16,
        style: Style,
    ) {
        let s = s.as_ref();
        let mut width = 0u16;

        for c in s.chars() {
            if width >= max_width {
                break;
            }
            if x >= self.area.x + self.area.width {
                break;
            }
            if y >= self.area.y + self.area.height {
                break;
            }

            let char_width = unicode_width(c);
            if width + char_width as u16 > max_width {
                break;
            }

            let cell = self.get_mut(x, y);
            cell.set_char(c).set_style(style);

            // For wide characters, mark the next cell as a continuation
            if char_width == 2 && x + 1 < self.area.x + self.area.width {
                let next_cell = self.get_mut(x + 1, y);
                next_cell.set_symbol("").set_style(style);
            }

            x += char_width as u16;
            width += char_width as u16;
        }
    }

    pub fn set_style(&mut self, area: super::render::Rect, style: Style) {
        let area = self.area.intersection(area);
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                self.get_mut(x, y).set_style(style);
            }
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
    }

    pub fn diff<'a>(&self, other: &'a Buffer) -> Vec<(u16, u16, &'a Cell)> {
        let mut updates = Vec::new();

        for y in self.area.y..self.area.y + self.area.height {
            for x in self.area.x..self.area.x + self.area.width {
                let current = self.get(x, y);
                let new = other.get(x, y);

                if current != new {
                    updates.push((x, y, new));
                }
            }
        }

        updates
    }
}

fn unicode_width(c: char) -> usize {
    if c.is_ascii() {
        1
    } else {
        // Simplified: treat most CJK as width 2, others as 1
        match c {
            '\u{1100}'..='\u{115F}' |
            '\u{2329}'..='\u{232A}' |
            '\u{2E80}'..='\u{303E}' |
            '\u{3040}'..='\u{A4CF}' |
            '\u{AC00}'..='\u{D7A3}' |
            '\u{F900}'..='\u{FAFF}' |
            '\u{FE10}'..='\u{FE19}' |
            '\u{FE30}'..='\u{FE6F}' |
            '\u{FF00}'..='\u{FF60}' |
            '\u{FFE0}'..='\u{FFE6}' |
            '\u{20000}'..='\u{2FFFD}' |
            '\u{30000}'..='\u{3FFFD}' => 2,
            _ => 1,
        }
    }
}
