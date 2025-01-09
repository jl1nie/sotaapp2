use domain::model::common::event::{FindAct, FindRef};
use domain::model::AwardProgram::{self, POTA, SOTA, WWFF};

pub fn findref_query_builder(mode: AwardProgram, r: &FindRef) -> String {
    let mut query: String = String::new();

    if let Some(refid) = &r.ref_id {
        if r.is_sota() && mode == SOTA {
            query.push_str(&format!("(summit_code = '{}') AND ", refid))
        } else if r.is_pota() && mode == POTA {
            query.push_str(&format!("(p.pota_code = '{}') AND ", refid))
        } else if r.is_wwff() && mode == WWFF {
            query.push_str(&format!("(p.wwff_code = '{}') AND ", refid))
        }
    }

    if let Some(name) = &r.name {
        if r.is_sota() && mode == SOTA {
            let name = format!("'{}%'", name);
            query.push_str(&format!(
                "(summit_code ILIKE {} OR summit_name LIKE {} OR summit_name_j LIKE {}) AND ",
                name, name, name
            ));
        } else {
            let name = format!("'{}%'", name);
            query.push_str(&format!(
                "(p.pota_code ILIKE {} OR p.wwff_code ILIKE {} OR p.park_name LIKE {} OR p.park_name_j LIKE {}) AND ",
                name, name, name, name
            ));
        }
    }

    if let Some(min_elev) = &r.min_elev {
        if r.is_sota() && mode == SOTA {
            query.push_str(&format!("(alt_m >= {}) AND ", min_elev));
        }
    }

    if let Some(min_area) = &r.min_area {
        if r.is_pota() && (mode == POTA || mode == WWFF) {
            query.push_str(&format!("(p.park_area >= {}) AND ", min_area));
        }
    }

    if let Some(bbox) = &r.bbox {
        query.push_str(&format!(
            "(ST_Within(coordinates, ST_MakeEnvelope({}, {}, {}, {}, 4326))) AND ",
            bbox.min_lon, bbox.min_lat, bbox.max_lon, bbox.max_lat
        ));
    }

    query.push_str("TRUE ");

    if r.is_sota() && mode == SOTA {
        if r.min_elev.is_some() {
            query.push_str("ORDER BY alt_m DESC ");
        } else {
            query.push_str("ORDER BY summit_code ");
        }
    } else if r.is_pota() && (mode == POTA || mode == WWFF) {
        if r.min_area.is_some() {
            query.push_str("ORDER BY p.park_area DESC ");
        } else {
            query.push_str("ORDER BY p.pota_code ");
        }
    }

    if let Some(limit) = &r.limit {
        query.push_str(&format!("LIMIT {} ", limit));
    }

    if let Some(offset) = &r.offset {
        query.push_str(&format!("OFFSET {} ", offset));
    }

    query
}

pub fn findact_query_builder(is_alert: bool, r: &FindAct) -> String {
    let mut query: String = String::new();

    if let Some(prog) = &r.program {
        query.push_str(format!("program = {} AND ", prog.as_i32()).as_str());
    }

    if is_alert {
        if let Some(after) = r.after {
            query.push_str(&format!(
                "start_time >= '{}' ORDER BY start_time DESC ",
                after
            ));
        }
    } else if let Some(after) = r.after {
        query.push_str(&format!(
            "spot_time >= '{}' ORDER BY spot_time DESC ",
            after
        ));
    }

    if let Some(limit) = &r.limit {
        query.push_str(&format!("LIMIT {} ", limit));
    }

    if let Some(offset) = &r.offset {
        query.push_str(&format!("OFFSET {} ", offset));
    }
    query
}
