use rinex::{ionex::*, prelude::*};
use crate::plot::Context;
use plotly::{
    Plot, 
    Scatter, 
    ImageFormat,
    color::NamedColor,
    common::{Marker, MarkerSymbol},
    layout::{Center, DragMode, Mapbox, MapboxStyle, Margin},
    Layout, 
    ScatterMapbox,
};

pub fn plot_tec_map(ctx: &mut Context, borders: ((f32,f32),(f32,f32)), record: &Record) {
    let map_center = ((borders.1.0 - borders.0.0)/2.0, (borders.1.1 - borders.0.1)/2.0);

    let mut latitudes: Vec<f64> = Vec::new();
    let mut longitudes: Vec<f64> = Vec::new();
    for (e, (tec, _, _)) in record {
        for point in tec {
            latitudes.push(point.latitude.into());
            longitudes.push(point.longitude.into());
        }
        break; // only care about 1 epoch for this
    }

    let grid = ScatterMapbox::new(latitudes, longitudes)
        .marker(
            Marker::new()
                .size(5)
                .symbol(MarkerSymbol::Circle)
                .color(NamedColor::Black)
                .opacity(0.2));

    let layout = Layout::new()
        .drag_mode(DragMode::Zoom)
        .margin(
            Margin::new()
                .top(0)
                .left(0)
                .bottom(0)
                .right(0)
        )
        .mapbox(
            Mapbox::new()
                .style(MapboxStyle::OpenStreetMap)
                //.center(Center::new(45.5017, -73.5673))
                .center(Center::new(32.5, -40.0))
                .zoom(1)
        );

    let mut plot = Plot::new();
    plot.add_trace(grid);
    plot.set_layout(layout);

    plot.show();
}
