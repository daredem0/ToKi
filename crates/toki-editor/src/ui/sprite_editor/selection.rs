//! Selection state for the sprite canvas.

/// Per-pixel selection mask covering the full canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionMask {
    pub width: u32,
    pub height: u32,
    mask: Vec<bool>,
}

impl SelectionMask {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mask: vec![false; (width * height) as usize],
        }
    }

    pub fn is_selected(&self, x: u32, y: u32) -> bool {
        self.index(x, y).is_some_and(|idx| self.mask[idx])
    }

    pub fn set(&mut self, x: u32, y: u32, selected: bool) {
        if let Some(idx) = self.index(x, y) {
            self.mask[idx] = selected;
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.mask.iter().any(|selected| *selected)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.mask.fill(false);
    }

    pub fn select_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.fill_rect(x, y, width, height, true);
    }

    #[allow(dead_code)]
    pub fn deselect_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.fill_rect(x, y, width, height, false);
    }

    pub fn select_pixel(&mut self, x: u32, y: u32) {
        self.set(x, y, true);
    }

    #[allow(dead_code)]
    pub fn deselect_pixel(&mut self, x: u32, y: u32) {
        self.set(x, y, false);
    }

    pub fn bounding_rect(&self) -> Option<SpriteSelection> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found = false;

        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_selected(x, y) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    found = true;
                }
            }
        }

        found.then(|| SpriteSelection::new(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
    }

    pub fn union_with(&mut self, other: &Self) {
        if self.width != other.width || self.height != other.height {
            return;
        }
        for (dst, src) in self.mask.iter_mut().zip(other.mask.iter()) {
            *dst |= *src;
        }
    }

    pub fn subtract(&mut self, other: &Self) {
        if self.width != other.width || self.height != other.height {
            return;
        }
        for (dst, src) in self.mask.iter_mut().zip(other.mask.iter()) {
            if *src {
                *dst = false;
            }
        }
    }

    fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, selected: bool) {
        let x_end = x.saturating_add(width).min(self.width);
        let y_end = y.saturating_add(height).min(self.height);
        for py in y..y_end {
            for px in x..x_end {
                self.set(px, py, selected);
            }
        }
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.width && y < self.height).then_some((y * self.width + x) as usize)
    }
}

/// Selection rectangle in canvas pixel coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteSelection {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl SpriteSelection {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a pixel is within the selection
    #[allow(dead_code)]
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}
