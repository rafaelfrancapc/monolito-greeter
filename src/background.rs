//! Windows 11 style Gaussian-blurred ambient background rendered in native Cairo.
//! Uses multi-stop radial and linear gradients to simulate an authentic out-of-focus wallpaper
//! with soft feathered contours, deep sapphire blues, and zero artificial stroked borders.
use gtk::{cairo, prelude::*};

pub fn landscape() -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.set_draw_func(|_, cr, width, height| {
        cr.scale(width as f64 / 1440., height as f64 / 900.);

        // Base nocturnal deep blue canvas
        let base = cairo::LinearGradient::new(0., 0., 1440., 900.);
        base.add_color_stop_rgb(0., 0.020, 0.045, 0.095);   // #050b18
        base.add_color_stop_rgb(0.5, 0.035, 0.075, 0.160);  // #091329
        base.add_color_stop_rgb(1., 0.018, 0.038, 0.078);   // #050a14
        let _ = cr.set_source(&base);
        let _ = cr.paint();

        // Primary soft gaussian-like ambient bloom (center-right, behind user avatar)
        let bloom_center = cairo::RadialGradient::new(720., 450., 40., 720., 450., 700.);
        bloom_center.add_color_stop_rgba(0., 0.10, 0.32, 0.68, 0.50);
        bloom_center.add_color_stop_rgba(0.35, 0.06, 0.20, 0.48, 0.32);
        bloom_center.add_color_stop_rgba(0.70, 0.03, 0.09, 0.25, 0.14);
        bloom_center.add_color_stop_rgba(1., 0.015, 0.04, 0.10, 0.);
        let _ = cr.set_source(&bloom_center);
        let _ = cr.paint();

        // Secondary soft upper ambient blur
        let bloom_top = cairo::RadialGradient::new(450., 220., 20., 450., 220., 550.);
        bloom_top.add_color_stop_rgba(0., 0.08, 0.24, 0.52, 0.38);
        bloom_top.add_color_stop_rgba(0.5, 0.04, 0.12, 0.30, 0.18);
        bloom_top.add_color_stop_rgba(1., 0.01, 0.03, 0.08, 0.);
        let _ = cr.set_source(&bloom_top);
        let _ = cr.paint();

        // Soft defocused ribbon wave 1 (sweeping background fold)
        let wave1 = cairo::LinearGradient::new(200., 300., 1100., 850.);
        wave1.add_color_stop_rgba(0., 0.06, 0.18, 0.40, 0.65);
        wave1.add_color_stop_rgba(0.5, 0.04, 0.12, 0.28, 0.70);
        wave1.add_color_stop_rgba(1., 0.02, 0.05, 0.14, 0.80);
        let _ = cr.set_source(&wave1);
        cr.move_to(100., 900.);
        cr.curve_to(260., 600., 380., 420., 640., 360.);
        cr.curve_to(880., 300., 1100., 400., 1260., 580.);
        cr.curve_to(1360., 700., 1410., 800., 1440., 900.);
        cr.line_to(100., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Soft defocused ribbon wave 2 (midground luminous curve)
        let wave2 = cairo::LinearGradient::new(350., 400., 950., 850.);
        wave2.add_color_stop_rgba(0., 0.12, 0.36, 0.72, 0.60);
        wave2.add_color_stop_rgba(0.4, 0.08, 0.24, 0.54, 0.65);
        wave2.add_color_stop_rgba(0.8, 0.03, 0.11, 0.28, 0.75);
        wave2.add_color_stop_rgba(1., 0.015, 0.05, 0.14, 0.85);
        let _ = cr.set_source(&wave2);
        cr.move_to(200., 900.);
        cr.curve_to(340., 680., 440., 500., 680., 430.);
        cr.curve_to(880., 370., 1040., 440., 1180., 600.);
        cr.curve_to(1250., 690., 1280., 800., 1320., 900.);
        cr.line_to(200., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Soft defocused ribbon wave 3 (foreground billowing curve)
        let wave3 = cairo::LinearGradient::new(200., 600., 1200., 900.);
        wave3.add_color_stop_rgba(0., 0.08, 0.26, 0.55, 0.65);
        wave3.add_color_stop_rgba(0.5, 0.04, 0.14, 0.34, 0.75);
        wave3.add_color_stop_rgba(1., 0.02, 0.06, 0.16, 0.88);
        let _ = cr.set_source(&wave3);
        cr.move_to(0., 900.);
        cr.curve_to(160., 770., 340., 680., 560., 660.);
        cr.curve_to(760., 640., 940., 730., 1160., 690.);
        cr.curve_to(1300., 660., 1380., 720., 1440., 770.);
        cr.line_to(1440., 900.);
        cr.line_to(0., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Soft defocused pocket curve (bottom-left)
        let wave4 = cairo::LinearGradient::new(0., 600., 500., 900.);
        wave4.add_color_stop_rgba(0., 0.04, 0.12, 0.28, 0.70);
        wave4.add_color_stop_rgba(0.6, 0.02, 0.06, 0.15, 0.82);
        wave4.add_color_stop_rgba(1., 0.01, 0.03, 0.08, 0.92);
        let _ = cr.set_source(&wave4);
        cr.move_to(0., 680.);
        cr.curve_to(140., 650., 260., 750., 380., 790.);
        cr.curve_to(510., 830., 640., 870., 780., 900.);
        cr.line_to(0., 900.);
        cr.close_path();
        let _ = cr.fill();

        // Edge vignette for natural focus
        let vignette = cairo::RadialGradient::new(720., 450., 450., 720., 450., 900.);
        vignette.add_color_stop_rgba(0., 0., 0., 0., 0.);
        vignette.add_color_stop_rgba(0.6, 0.01, 0.02, 0.05, 0.20);
        vignette.add_color_stop_rgba(1., 0.01, 0.02, 0.04, 0.55);
        let _ = cr.set_source(&vignette);
        let _ = cr.paint();
    });
    area
}
