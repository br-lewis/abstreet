use crate::app::App;
use crate::common::{ColorLegend, ColorNetwork};
use crate::layer::{Layer, LayerOutcome};
use abstutil::{prettyprint_usize, Counter};
use ezgui::{
    hotkey, Btn, Checkbox, Composite, Drawable, EventCtx, GfxCtx, HorizontalAlignment, Key, Line,
    Outcome, Text, TextExt, VerticalAlignment, Widget,
};
use geom::Time;
use map_model::{BuildingID, Map, ParkingLotID, RoadID};
use sim::{ParkingSpot, VehicleType};
use std::collections::HashSet;

pub struct Occupancy {
    time: Time,
    onstreet: bool,
    garages: bool,
    lots: bool,
    private_bldgs: bool,
    unzoomed: Drawable,
    zoomed: Drawable,
    composite: Composite,
}

impl Layer for Occupancy {
    fn name(&self) -> Option<&'static str> {
        Some("parking occupancy")
    }
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut App,
        minimap: &Composite,
    ) -> Option<LayerOutcome> {
        if app.primary.sim.time() != self.time {
            *self = Occupancy::new(
                ctx,
                app,
                self.onstreet,
                self.garages,
                self.lots,
                self.private_bldgs,
            );
        }

        self.composite.align_above(ctx, minimap);
        match self.composite.event(ctx) {
            Some(Outcome::Clicked(x)) => match x.as_ref() {
                "close" => {
                    return Some(LayerOutcome::Close);
                }
                _ => unreachable!(),
            },
            None => {
                let new_onstreet = self.composite.is_checked("On-street spots");
                let new_garages = self.composite.is_checked("Public garages");
                let new_lots = self.composite.is_checked("Parking lots");
                let new_private_bldgs = self.composite.is_checked("Private buildings");
                if self.onstreet != new_onstreet
                    || self.garages != new_garages
                    || self.lots != new_lots
                    || self.private_bldgs != new_private_bldgs
                {
                    *self = Occupancy::new(
                        ctx,
                        app,
                        new_onstreet,
                        new_garages,
                        new_lots,
                        new_private_bldgs,
                    );
                    self.composite.align_above(ctx, minimap);
                }
            }
        }
        None
    }
    fn draw(&self, g: &mut GfxCtx, app: &App) {
        self.composite.draw(g);
        if g.canvas.cam_zoom < app.opts.min_zoom_for_detail {
            g.redraw(&self.unzoomed);
        } else {
            g.redraw(&self.zoomed);
        }
    }
    fn draw_minimap(&self, g: &mut GfxCtx) {
        g.redraw(&self.unzoomed);
    }
}

impl Occupancy {
    pub fn new(
        ctx: &mut EventCtx,
        app: &App,
        onstreet: bool,
        garages: bool,
        lots: bool,
        private_bldgs: bool,
    ) -> Occupancy {
        let (mut filled_spots, mut avail_spots) = app.primary.sim.get_all_parking_spots();
        let mut filled_private_spots = 0;
        let mut avail_private_spots = 0;
        filled_spots.retain(|spot| match spot {
            ParkingSpot::Onstreet(_, _) => onstreet,
            ParkingSpot::Offstreet(b, _) => {
                if app
                    .primary
                    .map
                    .get_b(*b)
                    .parking
                    .as_ref()
                    .unwrap()
                    .public_garage_name
                    .is_some()
                {
                    garages
                } else {
                    filled_private_spots += 1;
                    private_bldgs
                }
            }
            ParkingSpot::Lot(_, _) => lots,
        });
        avail_spots.retain(|spot| match spot {
            ParkingSpot::Onstreet(_, _) => onstreet,
            ParkingSpot::Offstreet(b, _) => {
                if app
                    .primary
                    .map
                    .get_b(*b)
                    .parking
                    .as_ref()
                    .unwrap()
                    .public_garage_name
                    .is_some()
                {
                    garages
                } else {
                    avail_private_spots += 1;
                    private_bldgs
                }
            }
            ParkingSpot::Lot(_, _) => lots,
        });

        let mut total_ppl = 0;
        let mut has_car = 0;
        for p in app.primary.sim.get_all_people() {
            total_ppl += 1;
            if p.vehicles
                .iter()
                .any(|v| v.vehicle_type == VehicleType::Car)
            {
                has_car += 1;
            }
        }

        let composite = Composite::new(
            Widget::col(vec![
                Widget::row(vec![
                    Widget::draw_svg(ctx, "../data/system/assets/tools/layers.svg")
                        .margin_right(10),
                    "Parking occupancy".draw_text(ctx),
                    Btn::plaintext("X")
                        .build(ctx, "close", hotkey(Key::Escape))
                        .align_right(),
                ]),
                Text::from_multiline(vec![
                    Line(format!(
                        "{:.0}% of the population owns a car",
                        if total_ppl == 0 {
                            0.0
                        } else {
                            100.0 * (has_car as f64) / (total_ppl as f64)
                        }
                    )),
                    Line(""),
                    Line(format!(
                        "{} public spots filled",
                        prettyprint_usize(filled_spots.len())
                    )),
                    Line(format!(
                        "{} public spots available ",
                        prettyprint_usize(avail_spots.len())
                    )),
                    Line(""),
                    Line(format!(
                        "{} private spots filled",
                        prettyprint_usize(filled_private_spots)
                    )),
                    Line(format!(
                        "{} private spots available ",
                        prettyprint_usize(avail_private_spots)
                    )),
                ])
                .draw(ctx)
                .margin_below(10),
                Checkbox::text(ctx, "On-street spots", None, onstreet).margin_below(5),
                Checkbox::text(ctx, "Public garages", None, garages).margin_below(5),
                Checkbox::text(ctx, "Parking lots", None, lots).margin_below(10),
                Checkbox::text(ctx, "Private buildings", None, private_bldgs).margin_below(10),
                ColorLegend::gradient(ctx, &app.cs.good_to_bad_red, vec!["0%", "100%"]),
            ])
            .padding(5)
            .bg(app.cs.panel_bg),
        )
        .aligned(HorizontalAlignment::Right, VerticalAlignment::Center)
        .build(ctx);

        let mut filled = Counter::new();
        let mut avail = Counter::new();
        let mut keys = HashSet::new();
        for spot in filled_spots {
            let loc = Loc::new(spot, &app.primary.map);
            keys.insert(loc);
            filled.inc(loc);
        }
        for spot in avail_spots {
            let loc = Loc::new(spot, &app.primary.map);
            keys.insert(loc);
            avail.inc(loc);
        }

        let mut colorer = ColorNetwork::new(app);
        for loc in keys {
            let open = avail.get(loc);
            let closed = filled.get(loc);
            let percent = (closed as f64) / ((open + closed) as f64);
            let color = app.cs.good_to_bad_red.eval(percent);
            match loc {
                Loc::Road(r) => colorer.add_r(r, color),
                Loc::Bldg(b) => colorer.add_b(b, color),
                Loc::Lot(pl) => colorer.add_pl(pl, color),
            }
        }
        let (unzoomed, zoomed) = colorer.build(ctx);

        Occupancy {
            time: app.primary.sim.time(),
            onstreet,
            garages,
            lots,
            private_bldgs,
            unzoomed,
            zoomed,
            composite,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum Loc {
    Road(RoadID),
    Bldg(BuildingID),
    Lot(ParkingLotID),
}

impl Loc {
    fn new(spot: ParkingSpot, map: &Map) -> Loc {
        match spot {
            ParkingSpot::Onstreet(l, _) => Loc::Road(map.get_l(l).parent),
            ParkingSpot::Offstreet(b, _) => Loc::Bldg(b),
            ParkingSpot::Lot(pl, _) => Loc::Lot(pl),
        }
    }
}
