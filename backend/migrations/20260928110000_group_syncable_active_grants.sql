-- Active grants view: tracking rows whose owning group AND owning syncable
-- are both alive. Role convergence, sync-granted detection, and revocation
-- decisions must only consider active grants — otherwise a soft-deleted
-- group or an unlinked syncable keeps conferring its role (a revoked admin
-- grant would persist in the union forever).
CREATE VIEW group_syncable_active_grants AS
SELECT
    gsm.group_id,
    gsm.syncable_type,
    gsm.syncable_id,
    gsm.target_type,
    gsm.target_id,
    gsm.user_id,
    gsm.role,
    gsm.created_at
FROM group_syncable_memberships gsm
JOIN groups g
    ON g.id = gsm.group_id
   AND g.deleted_at IS NULL
JOIN group_syncables gs
    ON gs.group_id = gsm.group_id
   AND gs.syncable_type = gsm.syncable_type
   AND gs.syncable_id = gsm.syncable_id
   AND gs.delete_at IS NULL;
