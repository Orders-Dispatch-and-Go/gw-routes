use crate::model::Coord;

unsafe extern "C" {
    #[link_name = "compare"]
    fn ffi_compare(
        a: *const std::ffi::c_char,
        a_len: i32,
        b: *const std::ffi::c_char,
        b_len: i32,
    ) -> std::ffi::c_float;
}

pub fn compare(way1: &[Coord], way2: &[Coord]) -> f32 {
    let way_to_string = |way: &[Coord]| {
        way.iter()
            .map(|coord| format!("{:.6};{:.6}", coord.lat, coord.lon))
            .collect::<Vec<_>>()
            .join("|")
    };

    let way1_str = way_to_string(way1);
    let way2_str = way_to_string(way2);

    let distance = unsafe {
        ffi_compare(
            way1_str.as_ptr() as *const std::ffi::c_char,
            way1.len() as i32,
            way2_str.as_ptr() as *const std::ffi::c_char,
            way2.len() as i32,
        )
    };

    distance as f32
}
