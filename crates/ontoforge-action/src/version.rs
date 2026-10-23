use std::collections::BTreeMap;

use ontoforge_query::{
    TokenCodec,
    cursor::{VersionClaims, VersionTokenCodec},
};
use ontoforge_schema::ir::ParamType;
use ontoforge_storage::commit::{ActionGuardRejection, ActionGuardView};

use crate::{criteria::Authorization, source::row_key};

pub(crate) fn authorize(
    authorization: Authorization<'_>,
    expected: &[BTreeMap<String, String>],
    view: &ActionGuardView,
    tokens: &TokenCodec,
) -> Result<(), ActionGuardRejection> {
    let Authorization {
        active,
        action,
        principal,
        scope,
        items,
        sources,
        ..
    } = authorization;
    let codec = VersionTokenCodec::new(tokens);
    for (item, versions) in items.iter().zip(expected) {
        for (name, token) in versions {
            let (type_name, pk) = action
                .parameters
                .iter()
                .find_map(|(param, def)| {
                    if param.as_str() != name {
                        return None;
                    }
                    let ParamType::ObjectReference(type_name) = &def.param_type else {
                        return None;
                    };
                    item.references.get(param).map(|pk| (type_name, pk))
                })
                .ok_or(ActionGuardRejection::VersionConflict)?;
            let read = view
                .reads
                .iter()
                .find(|read| &read.type_name == type_name && &read.pk == pk)
                .ok_or(ActionGuardRejection::VersionConflict)?;
            codec
                .verify(
                    token,
                    &VersionClaims {
                        principal,
                        scope,
                        object_type: type_name,
                        pk,
                        def_version: active.def_version,
                        source: sources.get(&row_key(type_name, pk)),
                        edit: (read.row.version > 0).then_some(&read.row),
                    },
                )
                .map_err(|_| ActionGuardRejection::VersionConflict)?;
        }
    }
    Ok(())
}
