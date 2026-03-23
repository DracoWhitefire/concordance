mod color;
mod dsc;
mod frl;
mod timing;

pub(super) use color::{check_bit_depth, check_color_encoding};
pub(super) use dsc::check_dsc;
pub(super) use frl::check_frl_ceiling;
pub(super) use timing::{check_refresh_rate_range, check_tmds_clock_ceiling};
