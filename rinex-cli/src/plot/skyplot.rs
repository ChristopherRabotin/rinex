use super::PlotContext;
use crate::Context;
use plotly::{
    common::{Mode, Visible},
    ScatterPolar,
};
/*
 * Skyplot view
 */
pub fn skyplot(ctx: &Context, plot_ctx: &mut PlotContext) {
    plot_ctx.add_polar2d_plot("Skyplot");
    if let Some(ref nav) = ctx.nav_rinex {
        /*
         * "advanced" skyplot view,
         * observations were provided
         * color gradient emphasizes the SSI[dB]
         */
        if !nav.is_navigation_rinex() {
            println!("--nav should be Navigation Data!");
            return;
        }

        let sat_angles = nav.navigation_sat_angles(ctx.ground_position);
        for (index, (sv, epochs)) in sat_angles.iter().enumerate() {
            let el: Vec<f64> = epochs
                .iter()
                .map(|(_, (el, _))| el * 360.0 / std::f64::consts::PI)
                .collect();
            let azi: Vec<f64> = epochs
                .iter()
                .map(|(_, (_, azi))| azi * 360.0 / std::f64::consts::PI)
                .collect();
            let trace = ScatterPolar::new(el, azi)
                .mode(Mode::LinesMarkers)
                .visible({
                    if index < 4 {
                        Visible::True
                    } else {
                        Visible::LegendOnly
                    }
                })
                .name(sv.to_string());
            plot_ctx.add_trace(trace);
        }
    } else {
        /*
         * "simplified" skyplot view,
         * color gradient emphasizes the epoch/timestamp
         */
        let sat_angles = ctx.primary_rinex.navigation_sat_angles(ctx.ground_position);
        for (index, (sv, epochs)) in sat_angles.iter().enumerate() {
            let el: Vec<f64> = epochs
                .iter()
                .map(|(_, (el, _))| el * 360.0 / std::f64::consts::PI)
                .collect();
            let azi: Vec<f64> = epochs
                .iter()
                .map(|(_, (_, azi))| azi * 360.0 / std::f64::consts::PI)
                .collect();
            let trace = ScatterPolar::new(el, azi)
                .mode(Mode::LinesMarkers)
                .web_gl_mode(true)
                .visible({
                    if index < 4 {
                        Visible::True
                    } else {
                        Visible::LegendOnly
                    }
                })
                .name(sv.to_string());
            plot_ctx.add_trace(trace);
        }
    }
}
