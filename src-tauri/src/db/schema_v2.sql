-- v2: add new_content column to diff_proposals so the approval flow can
-- write the final file content on accept without re-parsing the unified
-- diff. Legacy rows created before v2 get NULL; the apply path treats
-- NULL as a not-applicable-legacy proposal and errors gracefully when
-- the user tries to approve such a proposal.
--
-- SQLite's ALTER TABLE ADD COLUMN errors if the column already exists,
-- so this migration relies on the runner only invoking it when
-- current_version < 2.
ALTER TABLE diff_proposals ADD COLUMN new_content TEXT;
