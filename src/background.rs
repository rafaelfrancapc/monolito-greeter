//! Windows 11 Bloom / macOS liquid curves procedural artwork in native Cairo.
//! Rendered entirely in vector math without remote images, WebKit or network dependency.
use gtk::{cairo, prelude::*};

pub fn landscape() -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.set_draw_func(|_, cr, width, height| {
        cr.scale(width as f64 / 1440., height as f64 / 900.);

        // Base nocturnal deep canvas
        let base = cairo::LinearGradient::new(0., 0., 1440., 900.);
        base.add_color_stop_rgb(0., 0.024, 0.047, 0.094);   // #060c18
        base.add_color_stop_rgb(0.5, 0.039, 0.078, 0.157);  // #0a1428
        base.add_color_stop_rgb(1., 0.020, 0.039, 0.078);   // #050a14
        let _ = cr.set_source(&base);
        let _ = cr.paint();

        // Primary ambient sapphire bloom behind center
        let bloom = cairo::RadialGradient::new(760., 450., 30., 760., 450., 680.);
        bloom.add_color_stop_rgba(0., 0.12, 0.35, 0.72, 0.52);
        bloom.add_color_stop_rgba(0.45, 0.07, 0.20, 0.45, 0.28);
        bloom.add_color_stop_rgba(1., 0.02, 0.05, 0.12, 0.);
        let _ = cr.set_source(&bloom);
        let _ = cr.paint();

        // Secondary soft glow for upper left
        let top_glow = cairo::RadialGradient::new(420., 260., 10., 420., 260., 520.);
        top_glow.add_color_stop_rgba(0., 0.08, 0.25, 0.55, 0.35);
        top_glow.add_color_stop_rgba(1., 0.02, 0.05, 0.10, 0.);
        let _ = cr.set_source(&top_glow);
        let _ = cr.paint();

        // Layer 1: Distant deep wave petals (Windows 11 Bloom inspired flow)
        let petal1 = cairo::LinearGradient::new(300., 200., 1100., 850.);
        petal1.add_color_stop_rgba(0., 0.05, 0.16, 0.36, 0.85);
        petal1.add_color_stop_rgba(0.6, 0.03, 0.09, 0.22, 0.90);
        petal1.add_color_stop_rgba(1., 0.015, 0.04, 0.10, 0.95);
        let _ = cr.set_source(&petal1);
        cr.move_to(120., 900.);
        cr.curve_to(280., 620., 380., 410., 620., 340.);
        cr.curve_to(880., 270., 1120., 380., 1280., 560.);
        cr.curve_to(1380., 680., 1420., 800., 1440., 900.);
        cr.line_to(120., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Layer 2: Main sweeping silk ribbons (left to center-right)
        let ribbon_main = cairo::LinearGradient::new(400., 350., 950., 800.);
        ribbon_main.add_color_stop_rgba(0., 0.14, 0.42, 0.82, 0.78);
        ribbon_main.add_color_stop_rgba(0.4, 0.09, 0.28, 0.60, 0.82);
        ribbon_main.add_color_stop_rgba(0.8, 0.04, 0.14, 0.32, 0.88);
        ribbon_main.add_color_stop_rgba(1., 0.02, 0.06, 0.15, 0.92);
        let _ = cr.set_source(&ribbon_main);
        cr.move_to(220., 900.);
        cr.curve_to(360., 700., 450., 500., 680., 420.);
        cr.curve_to(890., 350., 1050., 420., 1190., 580.);
        cr.curve_to(1260., 670., 1290., 790., 1320., 900.);
        cr.line_to(220., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Ribbon highlight edge (specular crest)
        let edge1 = cairo::LinearGradient::new(450., 500., 890., 350.);
        edge1.add_color_stop_rgba(0., 0.25, 0.60, 0.95, 0.05);
        edge1.add_color_stop_rgba(0.5, 0.38, 0.75, 1.0, 0.45);
        edge1.add_color_stop_rgba(1., 0.20, 0.50, 0.85, 0.05);
        let _ = cr.set_source(&edge1);
        cr.set_line_width(2.2);
        cr.move_to(360., 700.);
        cr.curve_to(450., 500., 680., 420., 890., 350.);
        cr.curve_to(1050., 420., 1190., 580., 1260., 670.);
        let _ = cr.stroke();

        // Layer 3: Dynamic billowing foreground wave (flowing across bottom)
        let fg_wave = cairo::LinearGradient::new(200., 550., 1100., 900.);
        fg_wave.add_color_stop_rgba(0., 0.09, 0.30, 0.62, 0.80);
        fg_wave.add_color_stop_rgba(0.5, 0.05, 0.18, 0.40, 0.85);
        fg_wave.add_color_stop_rgba(1., 0.02, 0.08, 0.18, 0.95);
        let _ = cr.set_source(&fg_wave);
        cr.move_to(0., 900.);
        cr.curve_to(150., 780., 320., 680., 540., 660.);
        cr.curve_to(760., 640., 940., 720., 1160., 680.);
        cr.curve_to(1300., 650., 1390., 710., 1440., 760.);
        cr.line_to(1440., 900.);
        cr.line_to(0., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Foreground crest highlight
        let edge2 = cairo::LinearGradient::new(320., 680., 1160., 680.);
        edge2.add_color_stop_rgba(0., 0.25, 0.62, 0.96, 0.10);
        edge2.add_color_stop_rgba(0.4, 0.42, 0.80, 1.0, 0.40);
        edge2.add_color_stop_rgba(0.8, 0.30, 0.68, 0.95, 0.25);
        edge2.add_color_stop_rgba(1., 0.15, 0.40, 0.75, 0.05);
        let _ = cr.set_source(&edge2);
        cr.set_line_width(1.8);
        cr.move_to(150., 780.);
        cr.curve_to(320., 680., 540., 660., 760., 640.);
        cr.curve_to(940., 720., 1160., 680., 1300., 650.);
        let _ = cr.stroke();

        // Layer 4: Deep folding pocket curve (bottom-left)
        let fold = cairo::LinearGradient::new(0., 600., 500., 900.);
        fold.add_color_stop_rgba(0., 0.04, 0.14, 0.30, 0.75);
        fold.add_color_stop_rgba(0.6, 0.02, 0.07, 0.16, 0.88);
        fold.add_color_stop_rgba(1., 0.01, 0.04, 0.09, 0.96);
        let _ = cr.set_source(&fold);
        cr.move_to(0., 660.);
        cr.curve_to(140., 630., 260., 740., 380., 780.);
        cr.curve_to(510., 820., 640., 860., 780., 900.);
        cr.line_to(0., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Edge vignette for soft contrast focus
        let vignette = cairo::RadialGradient::new(720., 450., 400., 720., 450., 880.);
        vignette.add_color_stop_rgba(0., 0., 0., 0., 0.);
        vignette.add_color_stop_rgba(0.7, 0.01, 0.02, 0.05, 0.25);
        vignette.add_color_stop_rgba(1., 0.01, 0.02, 0.04, 0.65);
        let _ = cr.set_source(&vignette);
        let _ = cr.paint();
    });
    area
}
