fn main() {
    // Generate the icon before compiling resources.
    let ico = std::path::Path::new("assets/palette.ico");
    if !ico.exists() {
        if let Err(e) = generate_icon(ico) {
            println!("cargo:warning=icon generation failed: {e}");
        }
    }

    println!("cargo:rerun-if-changed=assets/palette.rc");
    println!("cargo:rerun-if-changed=assets/palette.ico");
    println!("cargo:rerun-if-changed=assets/palette.manifest");

    if std::path::Path::new("assets/palette.rc").exists() {
        embed_resource::compile("assets/palette.rc", embed_resource::NONE);
    }
}

// ─── Icon generation ──────────────────────────────────────────────────────

fn generate_icon(path: &std::path::Path) -> std::io::Result<()> {
    let sizes: &[u32] = &[16, 24, 32, 48, 64, 128, 256];
    let mut images: Vec<(u32, Vec<u8>)> = Vec::new();
    for &s in sizes {
        let pixels = render_bgra(s);
        images.push((s, encode_bmp_ico(s, &pixels)));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, encode_ico(&images))
}

fn render_bgra(size: u32) -> Vec<[u8; 4]> {
    const SS: u32 = 4; // 4x4 supersample per pixel
    let s = size as f32;
    let mut out = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SS as f32;
                    let (cr, cg, cb, ca) = sample(px, py, s);
                    r += cr; g += cg; b += cb; a += ca;
                }
            }
            let n = (SS * SS) as f32;
            out.push([
                (b / n * 255.0).clamp(0.0, 255.0) as u8,
                (g / n * 255.0).clamp(0.0, 255.0) as u8,
                (r / n * 255.0).clamp(0.0, 255.0) as u8,
                (a / n * 255.0).clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    out
}

/// Draw a rounded blue square with a white magnifying glass.
/// Returns premultiplied-agnostic linear RGB in 0..1 plus alpha.
fn sample(x: f32, y: f32, s: f32) -> (f32, f32, f32, f32) {
    let pad = s * 0.05;
    let corner_r = s * 0.18;
    let inside_bg = rounded_rect_inside(x, y, pad, pad, s - pad * 2.0, s - pad * 2.0, corner_r);
    if !inside_bg {
        return (0.0, 0.0, 0.0, 0.0);
    }

    // Gradient background (top #4A7CF6 → bottom #2F5AE0).
    let t = (y / s).clamp(0.0, 1.0);
    let bg = (
        0.29 + (0.20 - 0.29) * t,
        0.49 + (0.35 - 0.49) * t,
        0.96 + (0.88 - 0.96) * t,
    );

    // Magnifying glass lens (ring).
    let lcx = s * 0.42;
    let lcy = s * 0.42;
    let lr = s * 0.24;
    let ring = s * 0.085;
    let d = ((x - lcx).powi(2) + (y - lcy).powi(2)).sqrt();
    let on_ring = (d - lr).abs() < ring / 2.0;

    // Handle from lens edge to bottom-right.
    let hx0 = lcx + lr * 0.7;
    let hy0 = lcy + lr * 0.7;
    let hx1 = s * 0.82;
    let hy1 = s * 0.82;
    let on_handle = line_thick(x, y, hx0, hy0, hx1, hy1, s * 0.075);

    if on_ring || on_handle {
        return (1.0, 1.0, 1.0, 1.0);
    }

    // Subtle glass tint so the lens interior doesn't flatten into the background.
    if d < lr - ring / 2.0 {
        return (
            bg.0 * 0.85 + 0.15,
            bg.1 * 0.85 + 0.15,
            bg.2 * 0.85 + 0.15,
            1.0,
        );
    }

    (bg.0, bg.1, bg.2, 1.0)
}

fn rounded_rect_inside(x: f32, y: f32, rx: f32, ry: f32, rw: f32, rh: f32, r: f32) -> bool {
    if x < rx || y < ry || x > rx + rw || y > ry + rh { return false; }
    // Flat horizontal band.
    if x >= rx + r && x <= rx + rw - r { return true; }
    // Flat vertical band.
    if y >= ry + r && y <= ry + rh - r { return true; }
    // Corner circle.
    let ccx = if x < rx + r { rx + r } else { rx + rw - r };
    let ccy = if y < ry + r { ry + r } else { ry + rh - r };
    let dx = x - ccx;
    let dy = y - ccy;
    dx * dx + dy * dy <= r * r
}

fn line_thick(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, half: f32) -> bool {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-6 { return false; }
    let t = ((x - x0) * dx + (y - y0) * dy) / len2;
    if t < 0.0 || t > 1.0 { return false; }
    let px = x0 + t * dx;
    let py = y0 + t * dy;
    ((x - px).powi(2) + (y - py).powi(2)).sqrt() <= half
}

// ─── ICO / BMP encoders ───────────────────────────────────────────────────

/// Encode one size as a BMP-in-ICO block (32-bit BGRA + 1bpp AND mask).
fn encode_bmp_ico(size: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    let (w, h) = (size, size);
    let mut out = Vec::with_capacity((40 + w * h * 4 + ((w + 31) / 32) * 4 * h) as usize);

    // BITMAPINFOHEADER
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&((h * 2) as i32).to_le_bytes()); // XOR + AND stacked
    out.extend_from_slice(&1u16.to_le_bytes());             // planes
    out.extend_from_slice(&32u16.to_le_bytes());            // bitCount
    out.extend_from_slice(&0u32.to_le_bytes());             // BI_RGB
    let and_row = ((w + 31) / 32) * 4;
    out.extend_from_slice(&(w * h * 4 + and_row * h).to_le_bytes()); // biSizeImage
    out.extend_from_slice(&0i32.to_le_bytes());             // xppm
    out.extend_from_slice(&0i32.to_le_bytes());             // yppm
    out.extend_from_slice(&0u32.to_le_bytes());             // clrUsed
    out.extend_from_slice(&0u32.to_le_bytes());             // clrImportant

    // XOR bitmap — bottom-up BGRA.
    for y in (0..h).rev() {
        for x in 0..w {
            let px = pixels[(y * w + x) as usize];
            out.push(px[0]);
            out.push(px[1]);
            out.push(px[2]);
            out.push(px[3]);
        }
    }

    // AND mask — 1bpp, bottom-up, 1 = transparent.
    for y in (0..h).rev() {
        let mut row = vec![0u8; and_row as usize];
        for x in 0..w {
            if pixels[(y * w + x) as usize][3] < 128 {
                row[(x / 8) as usize] |= 1 << (7 - (x % 8));
            }
        }
        out.extend_from_slice(&row);
    }

    out
}

/// Header + directory + concatenated BMPs.
fn encode_ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());             // reserved
    out.extend_from_slice(&1u16.to_le_bytes());             // type = icon
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = 6 + 16 * images.len() as u32;
    for (size, data) in images {
        let (w, h) = if *size >= 256 { (0u8, 0u8) } else { (*size as u8, *size as u8) };
        out.push(w);
        out.push(h);
        out.push(0);                                        // colorCount
        out.push(0);                                        // reserved
        out.extend_from_slice(&1u16.to_le_bytes());         // planes
        out.extend_from_slice(&32u16.to_le_bytes());        // bitCount
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in images {
        out.extend_from_slice(data);
    }
    out
}
