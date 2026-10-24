-- Fungsi pipa: transmisi atau distribusi. NULL dianggap distribusi sampai data diklasifikasikan.
ALTER TABLE features
    ADD COLUMN fungsi TEXT CHECK (fungsi IS NULL OR fungsi IN ('transmisi', 'distribusi'));
