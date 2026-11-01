//! Original procedural artwork: no remote images or network dependency at login.
use gtk::{cairo, prelude::*};

pub fn landscape() -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.set_draw_func(|_, cr, width, height| {
        cr.scale(width as f64 / 1440., height as f64 / 900.);
        let sky = cairo::LinearGradient::new(0., 0., 1200., 900.);
        sky.add_color_stop_rgb(0., 0.025, 0.065, 0.15);
        sky.add_color_stop_rgb(0.43, 0.08, 0.22, 0.32);
        sky.add_color_stop_rgb(0.77, 0.30, 0.46, 0.46);
        sky.add_color_stop_rgb(1., 0.44, 0.49, 0.44);
        let _ = cr.set_source(&sky);
        let _ = cr.paint();

        let glow = cairo::RadialGradient::new(1040., 360., 0., 1040., 360., 700.);
        glow.add_color_stop_rgba(0., 0.63, 0.83, 0.77, 0.33);
        glow.add_color_stop_rgba(1., 0.24, 0.6, 0.66, 0.);
        let _ = cr.set_source(&glow);
        let _ = cr.paint();

        // Distant ridges, a quiet horizon and large sculptural foregrounds.
        cr.move_to(0., 560.);
        cr.curve_to(250., 510., 350., 540., 555., 440.);
        cr.curve_to(700., 490., 750., 435., 890., 505.);
        cr.curve_to(1100., 430., 1260., 470., 1440., 390.);
        cr.line_to(1440., 900.);
        cr.line_to(0., 900.);
        cr.close_path();
        cr.set_source_rgba(0.09, 0.23, 0.27, 0.50);
        let _ = cr.fill();

        for i in 0..18 {
            let y = 560. + i as f64 * 13.;
            cr.set_source_rgba(0.62, 0.81, 0.77, 0.045 * (1. - i as f64 / 20.));
            cr.set_line_width(1.);
            cr.move_to(0., y);
            cr.line_to(1440., y);
            let _ = cr.stroke();
        }

        let left = cairo::LinearGradient::new(0., 410., 780., 940.);
        left.add_color_stop_rgb(0., 0.06, 0.15, 0.20);
        left.add_color_stop_rgb(0.5, 0.035, 0.10, 0.15);
        left.add_color_stop_rgb(1., 0.015, 0.045, 0.075);
        let _ = cr.set_source(&left);
        cr.move_to(0., 420.);
        cr.curve_to(170., 390., 190., 650., 390., 692.);
        cr.curve_to(560., 730., 640., 850., 900., 900.);
        cr.line_to(0., 900.);
        cr.close_path();
        let _ = cr.fill();

        let right = cairo::LinearGradient::new(1440., 450., 1000., 900.);
        right.add_color_stop_rgb(0., 0.16, 0.28, 0.29);
        right.add_color_stop_rgb(1., 0.025, 0.08, 0.12);
        let _ = cr.set_source(&right);
        cr.move_to(1440., 470.);
        cr.curve_to(1240., 580., 1350., 680., 1130., 730.);
        cr.curve_to(1020., 770., 950., 875., 900., 900.);
        cr.line_to(1440., 900.);
        cr.close_path();
        let _ = cr.fill();
    });
    area
}
