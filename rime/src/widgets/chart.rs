//! A minimal time-series line chart on an iced `Canvas` — the generic plotting
//! kernel. It draws axes and one stroked polyline per [`Series`].
//!
//! Hand-rolled rather than pulling a plotting crate: full control, no transitive
//! version coupling.
//!
//! The series colors are explicit (the caller picks them); the title and axis
//! colors come from the iced [`Theme`] the canvas is handed at draw time — *not*
//! [`crate::theme::tokens`]. `draw` runs during paint, after the host's palette
//! scope (see [`crate::theme::enter`]) has dropped, so reading the channel there
//! would be a use-after-scope. The canvas receives the live theme instead, which
//! stays coherent because the host derives that theme from the same palette.
//!
//! "What the axes mean" is the host's job: map your domain timeline into `(x, y)`
//! [`Series`] and hand them here.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

/// One labelled data series: points are `(x, y)` in data units.
pub struct Series {
    /// The points, in data units, drawn in order.
    pub points: Vec<(f64, f64)>,
    /// The polyline color (usually a [`crate::theme::Palette`] token).
    pub color: Color,
}

/// A line chart with a title and one or more series sharing axes. The x range is
/// the data's min..max; the y range is 0..max.
pub struct LineChart {
    /// Drawn at the top-left, in the theme's text color.
    pub title: String,
    /// One stroked polyline each, sharing the axes.
    pub series: Vec<Series>,
}

const MARGIN: f32 = 34.0;

impl<Message> canvas::Program<Message> for LineChart {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.palette();

        // Title.
        frame.fill_text(Text {
            content: self.title.clone(),
            position: Point::new(MARGIN, 4.0),
            color: palette.text,
            size: 14.0.into(),
            ..Text::default()
        });

        let w = bounds.width;
        let h = bounds.height;

        let xs: Vec<f64> = self
            .series
            .iter()
            .flat_map(|s| s.points.iter().map(|p| p.0))
            .collect();
        let ys: Vec<f64> = self
            .series
            .iter()
            .flat_map(|s| s.points.iter().map(|p| p.1))
            .collect();

        // Axes.
        let axis_color = Color {
            a: 0.4,
            ..palette.text
        };
        let axes = Path::new(|b| {
            b.move_to(Point::new(MARGIN, MARGIN));
            b.line_to(Point::new(MARGIN, h - MARGIN));
            b.line_to(Point::new(w - MARGIN, h - MARGIN));
        });
        frame.stroke(
            &axes,
            Stroke::default().with_color(axis_color).with_width(1.0),
        );

        if xs.is_empty() {
            return vec![frame.into_geometry()];
        }

        let xmin = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let xmax = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ymax = ys.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let xspan = (xmax - xmin).max(1e-9);

        let plot = |x: f64, y: f64| -> Point {
            let px = MARGIN + ((x - xmin) / xspan) as f32 * (w - 2.0 * MARGIN);
            let py = (h - MARGIN) - (y / ymax) as f32 * (h - 2.0 * MARGIN);
            Point::new(px, py)
        };

        // y-axis max label.
        frame.fill_text(Text {
            content: format!("{ymax:.0}"),
            position: Point::new(2.0, MARGIN - 6.0),
            color: axis_color,
            size: 11.0.into(),
            ..Text::default()
        });

        for series in &self.series {
            if series.points.len() < 2 {
                continue;
            }
            let line = Path::new(|b| {
                let mut points = series.points.iter();
                if let Some(&(x, y)) = points.next() {
                    b.move_to(plot(x, y));
                }
                for &(x, y) in points {
                    b.line_to(plot(x, y));
                }
            });
            frame.stroke(
                &line,
                Stroke::default().with_color(series.color).with_width(2.0),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Wrap a [`LineChart`] in a full-width canvas of the given pixel `height`, ready
/// to drop into a layout. For full control over sizing, hand the [`LineChart`]
/// (a `canvas::Program`) to `iced::widget::canvas` directly.
pub fn line_chart<'a, Message: 'a>(chart: LineChart, height: f32) -> Element<'a, Message> {
    iced::widget::canvas(chart)
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .into()
}
