use domain::model::{
    common::event::{FindAct, FindRef},
    AwardProgram,
};

pub fn findref_query_builder(r: &FindRef) -> String {
    let mut query: String = String::new();

    if let Some(refid) = &r.ref_id {
        if r.is_sota() {
            query.push_str(&format!("(summit_code = '{}') AND ", refid))
        } else {
            query.push_str(&format!("(park_reference = '{}') AND ", refid))
        }
    }

    if let Some(name) = &r.name {
        if r.is_sota() {
            let name = format!("'{}%'", name);
            query.push_str(&format!(
                "(summit_code ILIKE {} OR summit_name LIKE {} OR summit_name_j LIKE {}) AND",
                name, name, name
            ));
        } else {
            query.push_str(&format!(
                "(park_reference ILIKE {} OR park_name LIKE {} OR park_name_j LIKE {}) AND",
                name, name, name
            ));
        }
    }

    if let Some(min_elev) = &r.min_elev {
        if r.is_sota() {
            query.push_str(&format!("(alt_m >= {}) AND", min_elev));
        }
    }

    if let Some(min_area) = &r.min_area {
        if !r.is_sota() {
            query.push_str(&format!("(area >= {}) AND", min_area));
        }
    }

    if let Some(bbox) = &r.bbox {
        query.push_str(&format!(
            "(ST_Within(coordinates, ST_MakeEnvelope({}, {}, {}, {}, 4326))) AND",
            bbox.min_lon, bbox.min_lat, bbox.max_lon, bbox.max_lat
        ));
    }

    query.push_str(" TRUE ");

    if r.is_sota() {
        query.push_str("GROUP BY summit_code ");
    } else {
        query.push_str("GROUP BY park_reference ");
    }

    if r.min_elev.is_some() && r.is_sota() {
        query.push_str("ORDER BY alt_m DESC ");
    } else if r.min_area.is_some() {
        query.push_str("ORDER BY area DESC ");
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
        match prog {
            AwardProgram::SOTA => query.push_str("program = 0 AND "),
            AwardProgram::POTA => query.push_str("program = 1 AND "),
            AwardProgram::WWFF => query.push_str("program = 2 AND "),
        }
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
    tracing::info!("Query is {}", query);
    query
}
