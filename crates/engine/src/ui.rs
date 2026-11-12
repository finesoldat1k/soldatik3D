#[repr(C)]
#[derive(Clone, Copy)]
pub struct UiVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

pub struct Button {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub mode: u32,
    pub text: &'static str,
}

pub struct DebugMenu {
    pub is_open: bool,
    pub mode: u32,
    pub fps: f32,
    pub frame_time: f32,
    pub vertices: u32,
    pub triangles: u32,
    pub camera_pos: [f32; 3],
    pub buttons: Vec<Button>,
}

impl DebugMenu {
    pub fn new() -> Self {
        let mut buttons = Vec::new();
        // Кнопки выбора режима (создаются один раз)
        buttons.push(Button { x: 20.0, y: 175.0, w: 80.0, h: 25.0, mode: 0, text: "LIT" });
        buttons.push(Button { x: 110.0, y: 175.0, w: 80.0, h: 25.0, mode: 1, text: "NORMALS" });
        buttons.push(Button { x: 200.0, y: 175.0, w: 80.0, h: 25.0, mode: 2, text: "DEPTH" });

        Self { 
            is_open: false, mode: 0, fps: 0.0, frame_time: 0.0,
            vertices: 0, triangles: 0, camera_pos: [0.0; 3],
            buttons,
        }
    }

    pub fn generate_ui_vertices(&self, width: f32, height: f32) -> Vec<UiVertex> {
        let mut v = Vec::new();

        // Прицел
        let cx = width / 2.0; let cy = height / 2.0;
        add_rect(&mut v, width, height, cx - 2.0, cy - 2.0, 4.0, 4.0, [1.0, 1.0, 1.0, 0.7]);

        if self.is_open {
            let panel_w = 300.0;
            let panel_h = 290.0;
            
            // Фон
            add_rect(&mut v, width, height, 10.0, 10.0, panel_w, panel_h, [0.05, 0.05, 0.05, 0.9]);

            // Заголовок
            self.draw_text(&mut v, "SOLDATIK3D DEBUG", 20.0, 20.0, 2.0, [1.0, 0.8, 0.2, 1.0], width, height);
            add_rect(&mut v, width, height, 20.0, 35.0, panel_w - 20.0, 2.0, [1.0, 0.8, 0.2, 1.0]);

            // Статистика
            let stats = [
                format!("FPS: {:.0}", self.fps),
                format!("FRAME: {:.2} MS", self.frame_time),
                format!("VERTS: {}", self.vertices),
                format!("TRIS: {}", self.triangles),
                format!("CAM: {:.1} {:.1} {:.1}", self.camera_pos[0], self.camera_pos[1], self.camera_pos[2]),
            ];
            for (i, text) in stats.iter().enumerate() {
                self.draw_text(&mut v, text, 20.0, 45.0 + i as f32 * 20.0, 2.0, [1.0, 1.0, 1.0, 1.0], width, height);
            }

            // Кнопки
            for btn in &self.buttons {
                // Отрисовка фона кнопки (зеленая если активна, серая если нет)
                let color = if btn.mode == self.mode { [0.2, 0.8, 0.2, 1.0] } else { [0.3, 0.3, 0.3, 1.0] };
                add_rect(&mut v, width, height, btn.x, btn.y, btn.w, btn.h, color);
                
                // Отрисовка текста по центру кнопки
                let text_y = btn.y + 6.0;
                let text_x = btn.x + 6.0; // Грубое центрирование
                self.draw_text(&mut v, btn.text, text_x, text_y, 1.5, [1.0, 1.0, 1.0, 1.0], width, height);
            }

            add_rect(&mut v, width, height, 20.0, 210.0, panel_w - 20.0, 1.0, [0.5, 0.5, 0.5, 1.0]);

            // Подсказки
            self.draw_text(&mut v, "TAB: FREE MOUSE", 20.0, 220.0, 1.5, [0.7, 0.7, 0.7, 1.0], width, height);
            self.draw_text(&mut v, "WASD + SPACE / CTRL", 20.0, 235.0, 1.5, [0.7, 0.7, 0.7, 1.0], width, height);
            self.draw_text(&mut v, "SHIFT: SPRINT", 20.0, 250.0, 1.5, [0.7, 0.7, 0.7, 1.0], width, height);
        }

        v
    }

    // Проверка, попал ли клик по кнопке
    pub fn check_click(&mut self, mouse_x: f32, mouse_y: f32) -> bool {
        if !self.is_open { return false; }
        for btn in &self.buttons {
            if mouse_x >= btn.x && mouse_x <= btn.x + btn.w &&
               mouse_y >= btn.y && mouse_y <= btn.y + btn.h {
                self.mode = btn.mode;
                return true;
            }
        }
        false
    }

    fn draw_text(&self, v: &mut Vec<UiVertex>, text: &str, mut x: f32, y: f32, scale: f32, color: [f32; 4], width: f32, height: f32) {
        let char_size_uv = 1.0 / 16.0;
        for c in text.chars() {
            let index = (c as u8) as usize;
            if index < 32 || index > 126 { continue; }
            let cx = (index % 16) as f32;
            let cy = (index / 16) as f32;
            let u0 = cx * char_size_uv;
            let v0 = cy * char_size_uv;
            let ndc_x0 = (x / width) * 2.0 - 1.0;
            let ndc_y0 = (y / height) * 2.0 - 1.0;
            let ndc_x1 = ((x + 5.0 * scale) / width) * 2.0 - 1.0;
            let ndc_y1 = ((y + 7.0 * scale) / height) * 2.0 - 1.0;
            v.extend_from_slice(&[
                UiVertex { pos: [ndc_x0, ndc_y0], color, uv: [u0, v0] },
                UiVertex { pos: [ndc_x1, ndc_y0], color, uv: [u0 + char_size_uv, v0] },
                UiVertex { pos: [ndc_x1, ndc_y1], color, uv: [u0 + char_size_uv, v0 + char_size_uv] },
                UiVertex { pos: [ndc_x0, ndc_y0], color, uv: [u0, v0] },
                UiVertex { pos: [ndc_x1, ndc_y1], color, uv: [u0 + char_size_uv, v0 + char_size_uv] },
                UiVertex { pos: [ndc_x0, ndc_y1], color, uv: [u0, v0 + char_size_uv] },
            ]);
            x += 6.0 * scale;
        }
    }
}

fn add_rect(v: &mut Vec<UiVertex>, width: f32, height: f32, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let ndc_x0 = (x / width) * 2.0 - 1.0;
    let ndc_y0 = (y / height) * 2.0 - 1.0;
    let ndc_x1 = ((x + w) / width) * 2.0 - 1.0;
    let ndc_y1 = ((y + h) / height) * 2.0 - 1.0;
    v.extend_from_slice(&[
        UiVertex { pos: [ndc_x0, ndc_y0], color, uv: [0.0, 0.0] },
        UiVertex { pos: [ndc_x1, ndc_y0], color, uv: [0.0, 0.0] },
        UiVertex { pos: [ndc_x1, ndc_y1], color, uv: [0.0, 0.0] },
        UiVertex { pos: [ndc_x0, ndc_y0], color, uv: [0.0, 0.0] },
        UiVertex { pos: [ndc_x1, ndc_y1], color, uv: [0.0, 0.0] },
        UiVertex { pos: [ndc_x0, ndc_y1], color, uv: [0.0, 0.0] },
    ]);
}

// (Генератор шрифта, оставьте без изменений, он правильный)
pub fn generate_font_atlas() -> Vec<u8> {
    let mut pixels = vec![0u8; 128 * 128 * 4];
    let get_char_data = |c: char| -> [u8; 7] {
        match c {
            '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
            '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
            '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
            'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
            'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
            'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
            'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
            'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
            '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
            '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
            '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
            ' ' => [0,0,0,0,0,0,0],
            _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
        }
    };
    for c in 32..127 {
        let char_code = c as u8 as char;
        let data = get_char_data(char_code);
        let cx = (c % 16) * 8;
        let cy = (c / 16) * 8;
        for row in 0..7 {
            for col in 0..5 {
                if (data[row] & (1 << (4 - col))) != 0 {
                    let px = cx + col;
                    let py = cy + row;
                    let idx = (py * 128 + px) * 4;
                    pixels[idx] = 255; pixels[idx + 1] = 255; pixels[idx + 2] = 255; pixels[idx + 3] = 255;
                }
            }
        }
    }
    pixels
}
