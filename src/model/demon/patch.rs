use super::{Demon, DemonWithCreatorsAndRecords};
use crate::{
    model::player::Player,
    operation::{deserialize_non_optional, deserialize_optional, Get, Hotfix, Patch},
    permissions::PermissionsSet,
    schema::demons,
    Result,
};
use diesel::{Connection, ExpressionMethods, PgConnection, RunQueryDsl};
use log::info;
use serde_derive::Deserialize;

make_patch! {
    struct PatchDemon {
        name: String,
        position: i16,
        video: Option<String>,
        requirement: i16,
        verifier: String,
        publisher: String
    }
}

impl Hotfix for PatchDemon {
    fn required_permissions(&self) -> PermissionsSet {
        perms!(ListModerator or ListAdministrator)
    }
}

impl Patch<PatchDemon> for Demon {
    fn patch(mut self, mut patch: PatchDemon, connection: &PgConnection) -> Result<Self> {
        info!("Patching demon {} with {}", self.name, patch);

        validate_db!(patch, connection: Demon::validate_name[name], Demon::validate_position[position]);
        validate_nullable!(patch: Demon::validate_video[video]);

        let map = |name: &str| Player::get(name, connection);

        patch!(self, patch: name, video, requirement);
        try_map_patch!(self, patch: map => verifier, map => publisher);

        // We cannot move the PatchDemon object into the closure because we already moved data out
        // of it
        let position = patch.position;

        connection.transaction(move || {
            if let Some(position) = position {
                self.mv(position, connection)?
            }

            // alright, diesel::update(self) errors out for some reason
            diesel::update(demons::table)
                .filter(demons::name.eq(&self.name))
                .set((
                    demons::name.eq(&self.name),
                    demons::video.eq(&self.video),
                    demons::requirement.eq(&self.requirement),
                    demons::verifier.eq(&self.verifier.id),
                    demons::publisher.eq(&self.publisher.id),
                ))
                .execute(connection)?;

            Ok(self)
        })
    }

    fn permissions_for(&self, _: &PatchDemon) -> PermissionsSet {
        perms!(ListModerator or ListAdministrator)
    }
}

impl Patch<PatchDemon> for DemonWithCreatorsAndRecords {
    fn patch(self, patch: PatchDemon, connection: &PgConnection) -> Result<Self> {
        let DemonWithCreatorsAndRecords {
            demon,
            creators,
            records,
        } = self;

        let demon = demon.patch(patch, connection)?;

        Ok(DemonWithCreatorsAndRecords {
            demon,
            creators,
            records,
        })
    }

    fn permissions_for(&self, _: &PatchDemon) -> PermissionsSet {
        perms!(ListModerator or ListAdministrator)
    }
}
