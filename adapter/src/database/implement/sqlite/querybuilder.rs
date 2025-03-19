use domain::model::id::LogId;
use sqlx::query_builder::QueryBuilder;

use common::utils::calculate_bounding_box;
use domain::model::event::{CenterRadius, FindAct, FindLog, FindRef};
use domain::model::AwardProgram::{self, POTA, SOTA, WWFF};
use sqlx::Sqlite;

pub fn findref_query_builder<'a>(
    mode: AwardProgram,
    logid: Option<LogId>,
    query: &str,
    r: &FindRef,
) -> QueryBuilder<'a, Sqlite> {
    let mut builder = QueryBuilder::new(query);

    if let Some(logid) = logid {
        builder.push_bind(logid.raw());
        builder.push(" WHERE ");
    }

    if r.sota_code.is_some() && mode == SOTA {
        builder.push(" (summit_code = ");
        builder.push_bind(r.sota_code.clone().unwrap());
        builder.push(" ) AND ");
    } else if r.pota_code.is_some() && mode == POTA {
        builder.push(" (p.pota_code =");
        builder.push_bind(r.pota_code.clone().unwrap());
        builder.push(" ) AND ");
    } else if r.wwff_code.is_some() && mode == WWFF {
        builder.push("(p.wwff_code =");
        builder.push_bind(r.wwff_code.clone().unwrap());
        builder.push(" ) AND ");
    } else {
        if let Some(name) = &r.name {
            if r.is_sota() && mode == SOTA {
                builder.push(" (summit_code LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" OR summit_name LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" OR summit_name_j LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" ) AND ");
            } else {
                builder.push(" (p.pota_code LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" OR p.wwff_code LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" OR p.park_name LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" OR p.park_name_j LIKE ");
                builder.push_bind(format!("%{}%", name));
                builder.push(" ) AND ");
            }
        }

        if let Some(min_elev) = r.min_elev {
            if r.is_sota() && mode == SOTA {
                builder.push(" (alt_m >= ");
                builder.push_bind(min_elev);
                builder.push(" ) AND ");
            }
        }

        if let Some(min_area) = r.min_area {
            if r.is_pota() && (mode == POTA || mode == WWFF) {
                builder.push(" (p.park_area >= ");
                builder.push_bind(min_area);
                builder.push(" ) AND ");
            }
        }

        if let Some(bbox) = &r.bbox {
            builder.push(" (longitude BETWEEN ");
            builder.push_bind(bbox.min_lon);
            builder.push(" AND ");
            builder.push_bind(bbox.max_lon);
            builder.push(" AND ");
            builder.push(" latitude BETWEEN ");
            builder.push_bind(bbox.min_lat);
            builder.push(" AND ");
            builder.push_bind(bbox.max_lat);
            builder.push(" ) AND ");
        } else if let Some(CenterRadius { lon, lat, rad }) = r.center {
            let (min_lat, min_lon, max_lat, max_lon) = calculate_bounding_box(lat, lon, rad);

            builder.push(" (longitude BETWEEN ");
            builder.push_bind(min_lon);
            builder.push(" AND ");
            builder.push_bind(max_lon);
            builder.push(" AND ");
            builder.push(" latitude BETWEEN ");
            builder.push_bind(min_lat);
            builder.push(" AND ");
            builder.push_bind(max_lat);
            builder.push(" ) AND ");
        }
    }
    builder.push(" TRUE ");

    if r.is_sota() && mode == SOTA {
        if r.min_elev.is_some() {
            builder.push(" ORDER BY alt_m DESC ");
        } else {
            builder.push(" ORDER BY summit_code ");
        }
    } else if r.is_pota() && (mode == POTA || mode == WWFF) {
        if r.min_area.is_some() {
            builder.push(" ORDER BY p.park_area DESC ");
        } else {
            builder.push(" ORDER BY p.pota_code ");
        }
    }

    let max_limit = 300;

    if let Some(limit) = r.limit {
        builder.push(" LIMIT ");
        if limit < max_limit {
            builder.push_bind(limit);
        } else {
            builder.push_bind(max_limit);
        }
    } else {
        builder.push(" LIMIT ");
        builder.push_bind(max_limit);
    }

    if let Some(offset) = r.offset {
        builder.push(" OFFSET ");
        builder.push_bind(offset);
    }

    builder
}

pub fn findact_query_builder<'a>(
    is_alert: bool,
    query: &str,
    r: &FindAct,
) -> QueryBuilder<'a, Sqlite> {
    let mut builder = QueryBuilder::new(query);

    if let Some(prog) = &r.program {
        builder.push(" program = ");
        builder.push_bind(prog.as_i32());
        builder.push(" AND ");
    }

    if let Some(pat) = r.operator.clone() {
        builder.push(" operator = ");
        builder.push_bind(pat);
        builder.push(" AND ");
    }

    if is_alert {
        if let Some(after) = r.issued_after {
            builder.push(" start_time >= ");
            builder.push_bind(after);
            builder.push(" AND ");
        }
        builder.push("TRUE ORDER BY start_time ASC ");
    } else {
        if let Some(after) = r.issued_after {
            builder.push(" spot_time >= ");
            builder.push_bind(after);
            builder.push(" AND ");
        }
        builder.push(" TRUE ORDER BY spot_time DESC ");
    }

    if let Some(limit) = r.limit {
        builder.push(" LIMIT ");
        builder.push_bind(limit);
    }

    if let Some(offset) = r.offset {
        builder.push(" OFFSET ");
        builder.push_bind(offset);
    }

    builder
}

pub fn findlog_query_builder<'a>(query: &str, r: &FindLog) -> QueryBuilder<'a, Sqlite> {
    let mut builder = QueryBuilder::new(query);

    if r.activation {
        builder.push(" my_summit_code IS NOT NULL AND ");
    } else {
        builder.push(" my_summit_code IS NULL AND ");
    }

    if let Some(after) = r.after {
        builder.push(" time >= ");
        builder.push_bind(after);
        builder.push(" AND ");
    }

    if let Some(before) = r.before {
        builder.push(" time <= ");
        builder.push_bind(before);
        builder.push(" AND ");
    }

    builder.push(" TRUE ORDER BY time ASC ");

    builder
}
