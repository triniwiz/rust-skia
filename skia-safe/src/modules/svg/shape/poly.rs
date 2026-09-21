use crate::{
    Point,
    prelude::*,
    svg::{DebugAttributes, NodeSubtype},
};
use skia_bindings as sb;

pub type Poly = RCHandle<sb::SkSVGPoly>;

impl NodeSubtype for sb::SkSVGPoly {
    type Base = sb::SkSVGShape;
}

impl DebugAttributes for Poly {
    const NAME: &'static str = "Poly";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder.field("points", &self.points()));
    }
}

impl Poly {
    pub fn polygon() -> Self {
        Self::from_ptr(unsafe { sb::C_SkSVGPoly_MakePolygon() }).unwrap()
    }

    pub fn polyline() -> Self {
        Self::from_ptr(unsafe { sb::C_SkSVGPoly_MakePolyline() }).unwrap()
    }

    pub fn points(&self) -> &[Point] {
        unsafe {
            safer::from_raw_parts(
                Point::from_native_ptr(sb::C_SkSVGPoly_getPoints(self.native())),
                sb::C_SkSVGPoly_getPointsCount(self.native()),
            )
        }
    }

    pub fn set_points(&mut self, points: &[Point]) {
        unsafe {
            sb::C_SkSVGPoly_setPoints(
                self.native_mut(),
                points.as_ptr() as *const _,
                points.len(),
            )
        }
    }
}
