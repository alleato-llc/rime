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
/// the data's min..max; the y range is `0..y_max` (a fixed scale) or `0..data
/// peak` (auto).
pub struct LineChart {
    /// Drawn at the top-left, in the theme's text color.
    pub title: String,
    /// One stroked polyline each, sharing the axes.
    pub series: Vec<Series>,
    /// The top of the y axis. `None` auto-scales to the data's own peak (good
    /// for open-ended series like a byte-rate); `Some(m)` fixes the scale at
    /// `0..m` so a bounded gauge (a 0..100 percentage) reads as a fraction of
    /// its full range rather than filling the chart.
    pub y_max: Option<f64>,
    /// Label for the y-axis maximum. `None` shows the raw number (`{ymax:.0}`);
    /// `Some` shows this string instead — for pre-formatted units (e.g. a
    /// byte-rate like `8.0M/s`) the raw value can't express.
    pub y_max_label: Option<String>,
    /// Formats a point's `y` value for the on-hover readout. `None` shows the
    /// raw number (`{y:.0}`); `Some(f)` runs `f(y)` — for pre-formatted units
    /// the raw value can't express (a byte-rate, a percentage). A plain `fn`
    /// pointer (not a closure), so the unit is picked by the caller per chart.
    pub hover_format: Option<fn(f64) -> String>,
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
        cursor: mouse::Cursor,
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
        // A fixed scale when given (a bounded gauge), else the data's own peak.
        let ymax = self
            .y_max
            .unwrap_or_else(|| ys.iter().cloned().fold(0.0_f64, f64::max))
            .max(1.0);
        let xspan = (xmax - xmin).max(1e-9);

        let plot = |x: f64, y: f64| -> Point {
            let px = MARGIN + ((x - xmin) / xspan) as f32 * (w - 2.0 * MARGIN);
            let py = (h - MARGIN) - (y / ymax) as f32 * (h - 2.0 * MARGIN);
            Point::new(px, py)
        };

        // y-axis max label — the caller's pre-formatted string when given
        // (units the raw number can't express), else the raw maximum.
        frame.fill_text(Text {
            content: self
                .y_max_label
                .clone()
                .unwrap_or_else(|| format!("{ymax:.0}")),
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

        // Hover readout: when the pointer is over the plot, snap to the nearest
        // sample on each series (by plotted-x distance), draw a vertical guide
        // through it, and mark + label each series' value there in its own
        // units. Pure paint — it reads the `cursor` handed to `draw`, holding no
        // state and emitting no message.
        if let Some(pos) = cursor.position_in(bounds) {
            let in_plot =
                pos.x >= MARGIN && pos.x <= w - MARGIN && pos.y >= MARGIN && pos.y <= h - MARGIN;
            if in_plot {
                let fmt = |v: f64| {
                    self.hover_format
                        .map(|f| f(v))
                        .unwrap_or_else(|| format!("{v:.0}"))
                };
                let nearest = |s: &Series| -> Option<(f64, f64)> {
                    s.points
                        .iter()
                        .min_by(|a, b| {
                            let da = (plot(a.0, a.1).x - pos.x).abs();
                            let db = (plot(b.0, b.1).x - pos.x).abs();
                            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .copied()
                };

                // The guide sits at the first series' nearest sample; series
                // share the x axis, so every dot lands on it.
                if let Some((x, y)) = self.series.iter().find_map(nearest) {
                    let gx = plot(x, y).x;
                    let guide = Path::new(|b| {
                        b.move_to(Point::new(gx, MARGIN));
                        b.line_to(Point::new(gx, h - MARGIN));
                    });
                    frame.stroke(
                        &guide,
                        Stroke::default().with_color(axis_color).with_width(1.0),
                    );
                }

                for (k, series) in self.series.iter().enumerate() {
                    if let Some((x, y)) = nearest(series) {
                        let p = plot(x, y);
                        frame.fill(&Path::circle(p, 3.5), series.color);
                        // Alternate label above / below the dot so two series'
                        // readouts at the same x don't overprint.
                        let dy = if k % 2 == 0 { -15.0 } else { 6.0 };
                        frame.fill_text(Text {
                            content: fmt(y),
                            position: Point::new(
                                (p.x + 6.0).min(w - MARGIN - 4.0),
                                (p.y + dy).clamp(2.0, h - MARGIN),
                            ),
                            color: series.color,
                            size: 11.0.into(),
                            ..Text::default()
                        });
                    }
                }
            }
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

/// One series in a [`Sparkline`]: its samples and its line color.
#[derive(Debug, Clone)]
pub struct SparkSeries {
    /// The samples, oldest first. The newest sits at the right edge.
    pub values: Vec<f64>,
    /// Line color; the area is this color at low alpha.
    pub color: Color,
}

impl SparkSeries {
    /// A single-color series from its samples.
    pub fn new(values: Vec<f64>, color: Color) -> Self {
        Self { values, color }
    }
}

/// A compact filled area sparkline: one or more `series` (each oldest first,
/// newest at the right edge) drawn as small filled areas with stroked top lines,
/// no axes or labels — meant to sit inline in tight spots like a status bar. All
/// series share the value band `0..=max` (values are clamped into it) so they
/// read on a common scale; each carries its own color. Multiple series overlay
/// on the same canvas (e.g. disk read + write in one cell).
pub struct Sparkline {
    /// The overlaid series, drawn back-to-front.
    pub series: Vec<SparkSeries>,
    /// The top of the value band; values are clamped to `0..=max`.
    pub max: f64,
}

impl Sparkline {
    /// A single-series sparkline — the common case.
    pub fn single(values: Vec<f64>, max: f64, color: Color) -> Self {
        Self {
            series: vec![SparkSeries::new(values, color)],
            max,
        }
    }
}

impl<Message> canvas::Program<Message> for Sparkline {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let max = self.max.max(1e-9);

        for series in &self.series {
            let n = series.values.len();
            if n == 0 {
                continue;
            }
            let x_at = |i: usize| -> f32 {
                if n == 1 {
                    w
                } else {
                    (i as f32 / (n - 1) as f32) * w
                }
            };
            let y_at = |v: f64| -> f32 { h - (v / max).clamp(0.0, 1.0) as f32 * h };

            // Filled area under the line.
            let area = Path::new(|b| {
                b.move_to(Point::new(x_at(0), h));
                for (i, &v) in series.values.iter().enumerate() {
                    b.line_to(Point::new(x_at(i), y_at(v)));
                }
                b.line_to(Point::new(x_at(n - 1), h));
                b.close();
            });
            frame.fill(
                &area,
                Color {
                    a: 0.18,
                    ..series.color
                },
            );

            // Top line (needs at least two points to stroke).
            if n >= 2 {
                let line = Path::new(|b| {
                    b.move_to(Point::new(x_at(0), y_at(series.values[0])));
                    for (i, &v) in series.values.iter().enumerate().skip(1) {
                        b.line_to(Point::new(x_at(i), y_at(v)));
                    }
                });
                frame.stroke(
                    &line,
                    Stroke::default().with_color(series.color).with_width(1.5),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Wrap a [`Sparkline`] in a fixed-size canvas, ready to drop inline (e.g. into
/// a status-bar row).
pub fn sparkline<'a, Message: 'a>(
    spark: Sparkline,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    iced::widget::canvas(spark)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .into()
}
