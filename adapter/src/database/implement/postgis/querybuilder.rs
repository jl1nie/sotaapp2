use domain::model::event::{FindAct, FindRef};
use domain::model::AwardProgram::{self, POTA, SOTA, WWFF};
use sqlx::{Postgres, QueryBuilder};

/// SOTAリファレンス検索用のクエリビルダー
pub fn build_sota_ref_query<'a>(r: &'a FindRef) -> QueryBuilder<'a, Postgres> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT summit_code, association_name, region_name, summit_name, summit_name_j, \
         alt_m, alt_ft, grid_ref1, grid_ref2, coordinates, activation_count, activation_date, activation_call \
         FROM sota_summits WHERE ",
    );

    build_sota_conditions(&mut builder, r);
    builder
}

/// POTAリファレンス検索用のクエリビルダー
pub fn build_pota_ref_query<'a>(r: &'a FindRef) -> QueryBuilder<'a, Postgres> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT p.pota_code, p.wwff_code, p.park_name, p.park_name_j, p.park_location, p.park_locid, \
         p.park_type, p.park_inactive, p.park_area, p.coordinates, p.iota_reference, p.update_at, \
         p.first_act_date, p.first_act_callsign, p.activation_count \
         FROM pota_parks p WHERE ",
    );

    build_pota_conditions(&mut builder, r);
    builder
}

/// WWFF検索用のクエリビルダー
pub fn build_wwff_ref_query<'a>(r: &'a FindRef) -> QueryBuilder<'a, Postgres> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT p.pota_code, p.wwff_code, p.park_name, p.park_name_j, p.park_location, p.park_locid, \
         p.park_type, p.park_inactive, p.park_area, p.coordinates, p.iota_reference, p.update_at, \
         p.first_act_date, p.first_act_callsign, p.activation_count \
         FROM pota_parks p WHERE p.wwff_code IS NOT NULL AND ",
    );

    build_pota_conditions(&mut builder, r);
    builder
}

fn build_sota_conditions<'a>(builder: &mut QueryBuilder<'a, Postgres>, r: &'a FindRef) {
    if let Some(code) = &r.sota_code {
        builder.push("summit_code = ");
        builder.push_bind(code);
        builder.push(" AND ");
    } else {
        if let Some(name) = &r.name {
            let pattern = format!("%{}%", name);
            builder.push("(summit_code ILIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR summit_name LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR summit_name_j LIKE ");
            builder.push_bind(pattern);
            builder.push(") AND ");
        }

        if let Some(min_elev) = &r.min_elev {
            builder.push("alt_m >= ");
            builder.push_bind(*min_elev);
            builder.push(" AND ");
        }

        if let Some(bbox) = &r.bbox {
            builder.push("ST_Within(coordinates, ST_MakeEnvelope(");
            builder.push_bind(bbox.min_lon);
            builder.push(", ");
            builder.push_bind(bbox.min_lat);
            builder.push(", ");
            builder.push_bind(bbox.max_lon);
            builder.push(", ");
            builder.push_bind(bbox.max_lat);
            builder.push(", 4326)) AND ");
        } else if let Some(dist) = &r.dist {
            let lon = r.lon.unwrap_or_default();
            let lat = r.lat.unwrap_or_default();
            builder.push("ST_DWithin(coordinates, ST_GeogFromText('SRID=4326;POINT(' || ");
            builder.push_bind(lon);
            builder.push(" || ' ' || ");
            builder.push_bind(lat);
            builder.push(" || ')'), ");
            builder.push_bind(*dist);
            builder.push(") AND ");
        }
    }
    builder.push("TRUE ");

    if r.min_elev.is_some() {
        builder.push("ORDER BY alt_m DESC ");
    } else {
        builder.push("ORDER BY summit_code ");
    }

    if let Some(limit) = &r.limit {
        builder.push("LIMIT ");
        builder.push_bind(*limit);
        builder.push(" ");
    }

    if let Some(offset) = &r.offset {
        builder.push("OFFSET ");
        builder.push_bind(*offset);
    }
}

fn build_pota_conditions<'a>(builder: &mut QueryBuilder<'a, Postgres>, r: &'a FindRef) {
    if let Some(code) = &r.pota_code {
        builder.push("p.pota_code = ");
        builder.push_bind(code);
        builder.push(" AND ");
    } else if let Some(code) = &r.wwff_code {
        builder.push("p.wwff_code = ");
        builder.push_bind(code);
        builder.push(" AND ");
    } else {
        if let Some(name) = &r.name {
            let pattern = format!("%{}%", name);
            builder.push("(p.pota_code ILIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR p.wwff_code ILIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR p.park_name LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR p.park_name_j LIKE ");
            builder.push_bind(pattern);
            builder.push(") AND ");
        }

        if let Some(min_area) = &r.min_area {
            builder.push("p.park_area >= ");
            builder.push_bind(*min_area);
            builder.push(" AND ");
        }

        if let Some(bbox) = &r.bbox {
            builder.push("ST_Within(p.coordinates, ST_MakeEnvelope(");
            builder.push_bind(bbox.min_lon);
            builder.push(", ");
            builder.push_bind(bbox.min_lat);
            builder.push(", ");
            builder.push_bind(bbox.max_lon);
            builder.push(", ");
            builder.push_bind(bbox.max_lat);
            builder.push(", 4326)) AND ");
        } else if let Some(dist) = &r.dist {
            let lon = r.lon.unwrap_or_default();
            let lat = r.lat.unwrap_or_default();
            builder.push("ST_DWithin(p.coordinates, ST_GeogFromText('SRID=4326;POINT(' || ");
            builder.push_bind(lon);
            builder.push(" || ' ' || ");
            builder.push_bind(lat);
            builder.push(" || ')'), ");
            builder.push_bind(*dist);
            builder.push(") AND ");
        }
    }
    builder.push("TRUE ");

    if r.min_area.is_some() {
        builder.push("ORDER BY p.park_area DESC ");
    } else {
        builder.push("ORDER BY p.pota_code ");
    }

    if let Some(limit) = &r.limit {
        builder.push("LIMIT ");
        builder.push_bind(*limit);
        builder.push(" ");
    }

    if let Some(offset) = &r.offset {
        builder.push("OFFSET ");
        builder.push_bind(*offset);
    }
}

/// アクティベーション（アラート/スポット）検索用のクエリビルダー
pub fn build_activation_query<'a>(
    is_alert: bool,
    r: &'a FindAct,
    base_query: &str,
) -> QueryBuilder<'a, Postgres> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(base_query);
    builder.push(" WHERE ");

    if let Some(prog) = &r.program {
        builder.push("program = ");
        builder.push_bind(prog.as_i32());
        builder.push(" AND ");
    }

    if is_alert {
        if let Some(after) = r.issued_after {
            builder.push("start_time >= ");
            builder.push_bind(after);
            builder.push(" ORDER BY start_time ASC ");
        }
    } else if let Some(after) = r.issued_after {
        builder.push("spot_time >= ");
        builder.push_bind(after);
        builder.push(" ORDER BY spot_time DESC ");
    }

    if let Some(limit) = &r.limit {
        builder.push("LIMIT ");
        builder.push_bind(*limit);
        builder.push(" ");
    }

    if let Some(offset) = &r.offset {
        builder.push("OFFSET ");
        builder.push_bind(*offset);
    }

    builder
}

// レガシー互換: 既存コードから呼び出される関数（非推奨）
#[deprecated(note = "Use build_sota_ref_query or build_pota_ref_query instead")]
pub fn findref_query_builder(mode: AwardProgram, r: &FindRef) -> String {
    let mut query: String = String::new();

    if r.sota_code.is_some() && mode == SOTA {
        query.push_str(&format!(
            "(summit_code = '{}') AND ",
            r.sota_code.clone().unwrap()
        ))
    } else if r.pota_code.is_some() && mode == POTA {
        query.push_str(&format!(
            "(p.pota_code = '{}') AND ",
            r.pota_code.clone().unwrap()
        ))
    } else if r.wwff_code.is_some() && mode == WWFF {
        query.push_str(&format!(
            "(p.wwff_code = '{}') AND ",
            r.wwff_code.clone().unwrap()
        ))
    } else {
        if let Some(name) = &r.name {
            if r.is_sota() && mode == SOTA {
                let name = format!("'%{}%'", name);
                query.push_str(&format!(
                    "(summit_code ILIKE {} OR summit_name LIKE {} OR summit_name_j LIKE {}) AND ",
                    name, name, name
                ));
            } else {
                let name = format!("'%{}%'", name);
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
        } else if let Some(dist) = &r.dist {
            let lon = r.lon.unwrap_or_default();
            let lat = r.lat.unwrap_or_default();
            query.push_str(&format!(
                "(ST_DWithin(coordinates,ST_GeogFromText('SRID=4326;POINT({} {})'),{})) AND ",
                lon, lat, dist
            ));
        }
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

#[deprecated(note = "Use build_activation_query instead")]
pub fn findact_query_builder(is_alert: bool, r: &FindAct) -> String {
    let mut query: String = String::new();

    if let Some(prog) = &r.program {
        query.push_str(format!("program = {} AND ", prog.as_i32()).as_str());
    }

    if is_alert {
        if let Some(after) = r.issued_after {
            query.push_str(&format!(
                "start_time >= '{}' ORDER BY start_time ASC ",
                after
            ));
        }
    } else if let Some(after) = r.issued_after {
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
