//! Rebinding a vehicle onto a recreated live source (live-session reset).
//!
//! [`crate::scene3d::vehicle::VehicleConfig`] pins a vehicle to dense
//! source/field ids; a live-session reset tombstones them. When the
//! respawned link re-ingests the same messages, the config rebinds onto the
//! new ids by matching source label, topic name, and field name.

use delog_core::identity::{FieldId, SourceId};
use delog_core::snapshot::StoreSnapshot;

use super::{NedReference, OriMapping, PosMapping, VehicleConfig};

/// When this vehicle's source was tombstoned (its data dropped by a reset)
/// and exactly one live source with the same label re-ingests the same topic
/// and field names, rebind the config onto that source. The rebind is
/// atomic: it applies only when every mapped field reappeared. Returns true
/// when the config changed; false means the replacement source has not
/// fully reappeared yet (or does not exist).
pub fn rebind_to_recreated_source(config: &mut VehicleConfig, snapshot: &StoreSnapshot) -> bool {
    let Some(old) = snapshot.source(config.source) else {
        return false;
    };
    if !old.entry.removed {
        return false;
    }
    let label = old.entry.label.clone();
    let mut replacement = None;
    for source in snapshot
        .sources
        .iter()
        .filter(|source| !source.entry.removed)
    {
        if source.entry.label != label {
            continue;
        }
        if replacement.replace(source.entry.id).is_some() {
            return false; // two same-labeled sources are live: ambiguous
        }
    }
    let Some(new_source) = replacement else {
        return false;
    };
    let Some(pos) = recreated_pos(snapshot, &config.pos, new_source) else {
        return false;
    };
    let Some(ori) = recreated_ori(snapshot, &config.ori, new_source) else {
        return false;
    };
    config.source = new_source;
    config.pos = pos;
    config.ori = ori;
    true
}

/// The replacement for a tombstoned field: same topic and field name, under
/// the recreated source.
fn recreated_field(
    snapshot: &StoreSnapshot,
    old: FieldId,
    new_source: SourceId,
) -> Option<FieldId> {
    let old_field = snapshot.fields.get(old.index()).filter(|f| f.id == old)?;
    let old_topic = snapshot.topic(old_field.topic)?;
    let new_topic = snapshot
        .sources
        .iter()
        .find(|source| source.entry.id == new_source)?
        .topics
        .iter()
        .copied()
        .filter_map(|topic| snapshot.topic(topic))
        .find(|topic| !topic.entry.removed && topic.entry.name == old_topic.entry.name)?;
    snapshot
        .fields
        .iter()
        .find(|field| field.topic == new_topic.entry.id && field.name == old_field.name)
        .filter(|field| !field.removed)
        .map(|field| field.id)
}

fn recreated_reference(
    snapshot: &StoreSnapshot,
    reference: &Option<NedReference>,
    new_source: SourceId,
) -> Option<Option<NedReference>> {
    match reference {
        None => Some(None),
        Some(NedReference::Manual(geo)) => Some(Some(NedReference::Manual(*geo))),
        Some(NedReference::Fields { lat, lon, alt }) => Some(Some(NedReference::Fields {
            lat: recreated_field(snapshot, *lat, new_source)?,
            lon: recreated_field(snapshot, *lon, new_source)?,
            alt: recreated_field(snapshot, *alt, new_source)?,
        })),
    }
}

fn recreated_pos(
    snapshot: &StoreSnapshot,
    pos: &PosMapping,
    new_source: SourceId,
) -> Option<PosMapping> {
    match pos {
        PosMapping::Ned {
            north,
            east,
            down,
            reference,
        } => Some(PosMapping::Ned {
            north: recreated_field(snapshot, *north, new_source)?,
            east: recreated_field(snapshot, *east, new_source)?,
            down: recreated_field(snapshot, *down, new_source)?,
            reference: recreated_reference(snapshot, reference, new_source)?,
        }),
        PosMapping::Gps {
            lat,
            lon,
            alt,
            lat_lon_dege7,
            alt_mm,
            alt_offset_m,
        } => Some(PosMapping::Gps {
            lat: recreated_field(snapshot, *lat, new_source)?,
            lon: recreated_field(snapshot, *lon, new_source)?,
            alt: recreated_field(snapshot, *alt, new_source)?,
            lat_lon_dege7: *lat_lon_dege7,
            alt_mm: *alt_mm,
            alt_offset_m: *alt_offset_m,
        }),
    }
}

fn recreated_ori(
    snapshot: &StoreSnapshot,
    ori: &OriMapping,
    new_source: SourceId,
) -> Option<OriMapping> {
    match ori {
        OriMapping::Static => Some(OriMapping::Static),
        OriMapping::Euler {
            roll,
            pitch,
            yaw,
            degrees,
        } => Some(OriMapping::Euler {
            roll: recreated_field(snapshot, *roll, new_source)?,
            pitch: recreated_field(snapshot, *pitch, new_source)?,
            yaw: recreated_field(snapshot, *yaw, new_source)?,
            degrees: *degrees,
        }),
        OriMapping::Quat { w, x, y, z } => Some(OriMapping::Quat {
            w: recreated_field(snapshot, *w, new_source)?,
            x: recreated_field(snapshot, *x, new_source)?,
            y: recreated_field(snapshot, *y, new_source)?,
            z: recreated_field(snapshot, *z, new_source)?,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene3d::vehicle::{ModelKind, VehicleRuntime};
    use delog_core::identity::IdentityRegistry;
    use egui::Color32;

    fn ned_config(source: SourceId, fields: [FieldId; 3]) -> VehicleConfig {
        VehicleConfig {
            runtime: VehicleRuntime::unassigned(),
            source,
            label: "v".into(),
            show: true,
            show_path: true,
            pos: PosMapping::Ned {
                north: fields[0],
                east: fields[1],
                down: fields[2],
                reference: None,
            },
            ori: OriMapping::Static,
            model: ModelKind::Cone,
            color: Color32::WHITE,
            path_color: Color32::WHITE,
            scale: 1.0,
        }
    }

    #[test]
    fn rebind_to_recreated_source_remaps_source_and_fields_atomically() {
        use delog_core::identity::SourceKind;

        let label = "mavlink:udp:14550:sysid1";
        let mut identity = IdentityRegistry::new();
        let source = identity.add_source_with_kind(label, SourceKind::Live);
        let topic = identity.add_topic(source, "LOCAL_POSITION_NED").unwrap();
        let north = identity.add_field(topic, "x").unwrap();
        let east = identity.add_field(topic, "y").unwrap();
        let down = identity.add_field(topic, "z").unwrap();

        let mut config = ned_config(source, [north, east, down]);

        // Reset drops the source before its replacement exists: no rebind yet.
        identity.remove_source(source);
        let empty = StoreSnapshot::from_registry(&identity, [], 1).unwrap();
        assert!(!rebind_to_recreated_source(&mut config, &empty));
        assert_eq!(config.source, source);
        assert_eq!(config.pos, ned_config(source, [north, east, down]).pos);

        // The respawned link re-ingests the same topic and field names.
        let recreated = identity.add_source_with_kind(label, SourceKind::Live);
        let recreated_topic = identity.add_topic(recreated, "LOCAL_POSITION_NED").unwrap();
        let recreated_north = identity.add_field(recreated_topic, "x").unwrap();
        let recreated_east = identity.add_field(recreated_topic, "y").unwrap();
        let recreated_down = identity.add_field(recreated_topic, "z").unwrap();
        let snapshot = StoreSnapshot::from_registry(&identity, [], 2).unwrap();

        assert!(rebind_to_recreated_source(&mut config, &snapshot));
        assert_eq!(config.source, recreated);
        assert_eq!(
            config.pos,
            PosMapping::Ned {
                north: recreated_north,
                east: recreated_east,
                down: recreated_down,
                reference: None
            }
        );

        // An already-live vehicle is left alone.
        assert!(!rebind_to_recreated_source(&mut config, &snapshot));
    }

    #[test]
    fn rebind_is_atomic_when_a_field_has_not_reappeared() {
        use delog_core::identity::SourceKind;

        let label = "mavlink:udp:14550:sysid1";
        let mut identity = IdentityRegistry::new();
        let source = identity.add_source_with_kind(label, SourceKind::Live);
        let topic = identity.add_topic(source, "LOCAL_POSITION_NED").unwrap();
        let north = identity.add_field(topic, "x").unwrap();
        let east = identity.add_field(topic, "y").unwrap();
        let down = identity.add_field(topic, "z").unwrap();
        identity.remove_source(source);

        let mut config = ned_config(source, [north, east, down]);

        // The recreated source re-ingested only two of the three fields: the
        // rebind must not apply partially.
        let recreated = identity.add_source_with_kind(label, SourceKind::Live);
        let recreated_topic = identity.add_topic(recreated, "LOCAL_POSITION_NED").unwrap();
        let _ = identity.add_field(recreated_topic, "x").unwrap();
        let _ = identity.add_field(recreated_topic, "y").unwrap();
        let snapshot = StoreSnapshot::from_registry(&identity, [], 2).unwrap();

        assert!(!rebind_to_recreated_source(&mut config, &snapshot));
        assert_eq!(config.source, source);
        assert_eq!(config.pos, ned_config(source, [north, east, down]).pos);
    }

    #[test]
    fn rebind_remaps_the_fields_ned_reference() {
        use delog_core::identity::SourceKind;

        let label = "mavlink:udp:14550:sysid1";
        let mut identity = IdentityRegistry::new();
        let source = identity.add_source_with_kind(label, SourceKind::Live);
        let pos_topic = identity.add_topic(source, "LOCAL_POSITION_NED").unwrap();
        let north = identity.add_field(pos_topic, "x").unwrap();
        let east = identity.add_field(pos_topic, "y").unwrap();
        let down = identity.add_field(pos_topic, "z").unwrap();
        let gps_topic = identity.add_topic(source, "GPS_RAW_INT").unwrap();
        let ref_lat = identity.add_field(gps_topic, "lat").unwrap();
        let ref_lon = identity.add_field(gps_topic, "lon").unwrap();
        let ref_alt = identity.add_field(gps_topic, "alt").unwrap();
        identity.remove_source(source);

        let mut config = ned_config(source, [north, east, down]);
        config.pos = PosMapping::Ned {
            north,
            east,
            down,
            reference: Some(NedReference::Fields {
                lat: ref_lat,
                lon: ref_lon,
                alt: ref_alt,
            }),
        };

        let recreated = identity.add_source_with_kind(label, SourceKind::Live);
        let recreated_pos = identity.add_topic(recreated, "LOCAL_POSITION_NED").unwrap();
        let recreated_gps = identity.add_topic(recreated, "GPS_RAW_INT").unwrap();
        let new_north = identity.add_field(recreated_pos, "x").unwrap();
        let new_east = identity.add_field(recreated_pos, "y").unwrap();
        let new_down = identity.add_field(recreated_pos, "z").unwrap();
        let new_lat = identity.add_field(recreated_gps, "lat").unwrap();
        let new_lon = identity.add_field(recreated_gps, "lon").unwrap();
        let new_alt = identity.add_field(recreated_gps, "alt").unwrap();
        let snapshot = StoreSnapshot::from_registry(&identity, [], 2).unwrap();

        assert!(rebind_to_recreated_source(&mut config, &snapshot));
        let PosMapping::Ned {
            north,
            east,
            down,
            reference,
        } = &config.pos
        else {
            panic!("position stays a NED mapping");
        };
        assert_eq!((*north, *east, *down), (new_north, new_east, new_down));
        assert_eq!(
            reference,
            &Some(NedReference::Fields {
                lat: new_lat,
                lon: new_lon,
                alt: new_alt
            })
        );
    }
}
