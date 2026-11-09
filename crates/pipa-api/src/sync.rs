//! Sinkron referensi SPM air minum dari Arumanis: desa dan capaian (endpoint publik), unit SPAM dan
//! Pokmas (butuh `ARUMANIS_TOKEN`). Dipanggil oleh jadwal berkala, tombol admin, dan `pipa-api sync`.

use std::{env, sync::Arc, time::Duration};

use pipa_store::{ApplyResult, DesaRefInput, PekerjaanRefInput, PokmasRefInput, Store, UnitRefInput};
use pipa_sync::{derive_pokmas, ArumanisClient};
use serde::Serialize;
use tokio::sync::Mutex;

const DEFAULT_ARUMANIS_API: &str = "https://apiamis.cianjur.space/api";

/// Id desa di Arumanis yang tidak dipakai: entri duplikat atau tanpa data (ejaan lain dari desa yang sudah ada,
/// dan dua desa yang tidak ada di batas BPS). Tidak dimasukkan ke `ref_desa`, jadi ditandai tidak aktif.
const DESA_DIKECUALIKAN: [i64; 4] = [1000, 1001, 1002, 1003];

#[derive(Clone)]
pub struct SyncConfig {
    pub base_url: String,
    pub token: Option<String>,
    /// Kunci API layanan (`X-API-Key`, jalur `/public/v1/*`). Disarankan: tidak kedaluwarsa. Menang atas `token`.
    pub api_key: Option<String>,
    /// Jeda sinkron otomatis dalam menit. 0 mematikan sinkron otomatis.
    pub interval_menit: u64,
}

impl SyncConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: env::var("ARUMANIS_API_URL").unwrap_or_else(|_| DEFAULT_ARUMANIS_API.into()),
            token: env::var("ARUMANIS_TOKEN").ok().filter(|t| !t.trim().is_empty()),
            api_key: env::var("ARUMANIS_API_KEY").ok().filter(|t| !t.trim().is_empty()),
            interval_menit: env::var("ARUMANIS_SYNC_MENIT").ok().and_then(|v| v.parse().ok()).unwrap_or(60),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SyncReport {
    pub run_id: i64,
    pub status: &'static str,
    pub hasil: Option<ApplyResult>,
    pub pesan: Option<String>,
}

/// Sinkron lain sedang berjalan.
#[derive(Debug)]
pub struct SyncBusy;

#[derive(Clone)]
pub struct Syncer {
    store: Store,
    config: SyncConfig,
    lock: Arc<Mutex<()>>,
}

impl Syncer {
    pub fn new(store: Store, config: SyncConfig) -> Self {
        Self { store, config, lock: Arc::new(Mutex::new(())) }
    }

    pub fn interval_menit(&self) -> u64 {
        self.config.interval_menit
    }

    pub async fn run(&self, dipicu_oleh: &str) -> Result<SyncReport, SyncBusy> {
        let Ok(_guard) = self.lock.try_lock() else {
            return Err(SyncBusy);
        };

        let run_id = match self.store.start_sync_run(dipicu_oleh).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(error = %e, "tidak bisa mencatat sinkron");
                return Ok(SyncReport { run_id: 0, status: "gagal", hasil: None, pesan: Some(e.to_string()) });
            }
        };

        let (status, hasil, pesan) = match self.execute().await {
            Ok((hasil, catatan)) => {
                let status = if catatan.is_some() { "sebagian" } else { "berhasil" };
                (status, Some(hasil), catatan)
            }
            Err(e) => ("gagal", None, Some(e)),
        };

        if let Err(e) = self.store.finish_sync_run(run_id, status, hasil.as_ref(), pesan.as_deref()).await {
            tracing::error!(error = %e, run_id, "tidak bisa menutup catatan sinkron");
        }
        match status {
            "gagal" => tracing::warn!(run_id, pesan = ?pesan, "sinkron Arumanis gagal"),
            _ => tracing::info!(run_id, status, hasil = ?hasil, "sinkron Arumanis selesai"),
        }
        Ok(SyncReport { run_id, status, hasil, pesan })
    }

    async fn execute(&self) -> Result<(ApplyResult, Option<String>), String> {
        let client = ArumanisClient::with_credentials(
            &self.config.base_url,
            self.config.token.clone(),
            self.config.api_key.clone(),
        )
        .map_err(|e| e.to_string())?;

        let stats = client.desa_stats().await.map_err(|e| e.to_string())?;
        // Daftar kosong hampir pasti gangguan di Arumanis. Jangan sampai semua desa ditandai tidak aktif.
        if stats.is_empty() {
            return Err("Arumanis mengembalikan 0 desa; sinkron dibatalkan dan data lama dipertahankan".into());
        }
        let desa: Vec<DesaRefInput> = stats
            .into_iter()
            .filter(|s| !DESA_DIKECUALIKAN.contains(&s.desa_id))
            .map(|s| DesaRefInput {
                id: s.desa_id,
                nama: s.desa.unwrap_or_default(),
                kecamatan: s.kecamatan.unwrap_or_default(),
                target_kk: s.target,
                jp_kk: s.kk,
                bjp_master: s.bjp_master,
                bjp_unit: s.bjp_unit,
                sr: s.sr,
                jiwa: s.jiwa,
                unit_count: s.unit_count,
            })
            .collect();

        let mut catatan: Vec<String> = Vec::new();
        let units = if !client.has_credentials() {
            catatan.push("ARUMANIS_API_KEY (atau ARUMANIS_TOKEN) belum diatur: unit SPAM dan Pokmas tidak disinkron".into());
            None
        } else {
            match client.units().await {
                Ok(units) if units.is_empty() => {
                    catatan.push("Arumanis mengembalikan 0 unit SPAM; unit dan Pokmas tidak diperbarui".into());
                    None
                }
                Ok(units) => Some(units),
                Err(e) => {
                    catatan.push(format!("unit SPAM dan Pokmas tidak diperbarui: {e}"));
                    None
                }
            }
        };

        // Daftar pekerjaan terpisah: gagal di sini hanya dicatat, sinkron desa dan unit tetap berjalan.
        let pekerjaan: Option<Vec<PekerjaanRefInput>> = if !client.has_api_key() {
            catatan.push("ARUMANIS_API_KEY belum diatur: daftar pekerjaan tidak disinkron".into());
            None
        } else {
            match client.pekerjaan().await {
                Ok(list) if list.is_empty() => {
                    catatan.push("Arumanis mengembalikan 0 pekerjaan; daftar pekerjaan tidak diperbarui".into());
                    None
                }
                Ok(list) => Some(
                    list.into_iter()
                        .map(|p| PekerjaanRefInput {
                            id: p.id,
                            nama_paket: p.nama_paket,
                            tahun: p.tahun.and_then(|t| i16::try_from(t).ok()),
                            desa_id: p.desa_id,
                        })
                        .collect(),
                ),
                Err(e) => {
                    catatan.push(format!("daftar pekerjaan tidak diperbarui: {e}"));
                    None
                }
            }
        };

        let mapped = units.map(|units| {
            let pokmas: Vec<PokmasRefInput> = derive_pokmas(&units)
                .into_iter()
                .map(|p| PokmasRefInput {
                    desa_id: p.desa_id,
                    nama: p.nama,
                    perdes: p.perdes,
                    kepala: p.kepala,
                    bendahara: p.bendahara,
                    sekretaris: p.sekretaris,
                    konflik: p.konflik,
                    sumber_unit_ids: p.sumber_unit_ids,
                })
                .collect();
            let units: Vec<UnitRefInput> = units
                .iter()
                .map(|u| {
                    let c = u.capaian();
                    UnitRefInput {
                        id: u.id,
                        desa_id: u.desa_id,
                        nama: u.name.clone(),
                        sistem_layanan: u.sistem_layanan.clone(),
                        sumber_dana: u.sumber_dana.clone(),
                        is_simspam: u.is_simspam,
                        sr: c.sr,
                        kk: c.kk,
                        jiwa: c.jiwa,
                        bjp_kk: c.bjp_kk,
                    }
                })
                .collect();
            (units, pokmas)
        });

        let hasil = self
            .store
            .apply_sync(&desa, mapped.as_ref().map(|(u, p)| (u.as_slice(), p.as_slice())))
            .await
            .map_err(|e| e.to_string())?;
        if let Some(pekerjaan) = pekerjaan {
            if let Err(e) = self.store.apply_pekerjaan(&pekerjaan).await {
                catatan.push(format!("daftar pekerjaan gagal disimpan: {e}"));
            }
        }
        if hasil.unit_dilewati > 0 {
            catatan.push(format!("{} unit dilewati karena desanya tidak ada di daftar desa", hasil.unit_dilewati));
        }
        let catatan = (!catatan.is_empty()).then(|| catatan.join("; "));
        Ok((hasil, catatan))
    }

    /// Jalankan sinkron saat start, lalu setiap `interval_menit`. Tidak berjalan bila interval 0.
    pub fn spawn_schedule(&self) {
        let menit = self.config.interval_menit;
        if menit == 0 {
            tracing::info!("sinkron otomatis Arumanis dimatikan (ARUMANIS_SYNC_MENIT=0)");
            return;
        }
        let syncer = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(menit * 60));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                if syncer.run("jadwal").await.is_err() {
                    tracing::info!("sinkron terjadwal dilewati: sinkron lain sedang berjalan");
                }
            }
        });
    }
}
