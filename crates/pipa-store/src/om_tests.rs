//! Tes integrasi O&M. Butuh DATABASE_URL ke PostgreSQL dengan PostGIS (sqlx membuat database uji sementara).
//! Jalankan: `cargo test -p pipa-store -- --ignored om_`

use pipa_core::gangguan::StatusGangguan;
use pipa_core::om::OmError;
use pipa_core::rehab::StatusRehab;
use pipa_core::tanggal::Tanggal;
use sqlx::PgPool;

use crate::{FilterGangguan, GangguanBaru, JadwalBaru, Store, StoreError};

async fn jaringan_dan_aset(pool: &PgPool, kondisi: &str) -> (i64, i64) {
    let network: i64 = sqlx::query_scalar("INSERT INTO networks (nama) VALUES ('uji') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let feature: i64 = sqlx::query_scalar(
        "INSERT INTO features (network_id, feature_type, geom, kondisi) \
         VALUES ($1, 'pipa', ST_SetSRID(ST_GeomFromText('LINESTRING(107.1 -6.8, 107.11 -6.8)'), 4326), $2) \
         RETURNING id",
    )
    .bind(network)
    .bind(kondisi)
    .fetch_one(pool)
    .await
    .unwrap();
    (network, feature)
}

fn t(s: &str) -> Tanggal {
    Tanggal::parse(s).unwrap()
}

#[ignore = "butuh database PostgreSQL"]
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn om_gangguan_titik_dan_status_hanya_maju(pool: PgPool) {
    let (network, _) = jaringan_dan_aset(&pool, "baik").await;
    let store = Store::from_pool(pool);
    let baru = GangguanBaru {
        network_id: network,
        feature_id: None,
        titik: Some((107.1, -6.8)),
        jenis: "air_mati".into(),
        deskripsi: "Air tidak mengalir sejak pagi".into(),
        pelapor: "Siti".into(),
    };
    let g = store.buat_gangguan(baru, "operator").await.unwrap();
    assert_eq!(g.status, "dilaporkan");
    assert_eq!(g.lon, Some(107.1));

    let g = store.ubah_status_gangguan(g.id, "ditangani", "operator").await.unwrap();
    assert_eq!(g.status, "ditangani");
    // Status mundur ditolak sebagai aturan bisnis.
    let mundur = store.ubah_status_gangguan(g.id, "dilaporkan", "operator").await;
    assert!(matches!(mundur, Err(StoreError::Om(OmError::Aturan(_)))), "{mundur:?}");
    // Status yang sama tidak mengubah apa pun dan tidak menulis riwayat.
    let sama = store.ubah_status_gangguan(g.id, "ditangani", "operator").await.unwrap();
    assert_eq!(sama.status, "ditangani");
    let riwayat: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM om_riwayat WHERE sumber = 'gangguan' AND sumber_id = $1")
        .bind(g.id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(riwayat, 2, "tambah dan satu ubah status");
    assert_eq!(StatusGangguan::parse(&g.status).unwrap(), StatusGangguan::Ditangani);
}

#[ignore = "butuh database PostgreSQL"]
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn om_gangguan_aset_harus_di_jaringan_yang_sama(pool: PgPool) {
    let (_, feature) = jaringan_dan_aset(&pool, "baik").await;
    let jaringan_lain: i64 = sqlx::query_scalar("INSERT INTO networks (nama) VALUES ('lain') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let store = Store::from_pool(pool);
    let baru = GangguanBaru {
        network_id: jaringan_lain,
        feature_id: Some(feature),
        titik: None,
        jenis: "pipa_bocor".into(),
        deskripsi: "Bocor".into(),
        pelapor: "Budi".into(),
    };
    let hasil = store.buat_gangguan(baru, "operator").await;
    assert!(matches!(hasil, Err(StoreError::Om(OmError::Aturan(_)))), "{hasil:?}");
}

#[ignore = "butuh database PostgreSQL"]
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn om_foto_dibatasi_lima_per_laporan(pool: PgPool) {
    let (network, feature) = jaringan_dan_aset(&pool, "rusak_berat").await;
    let store = Store::from_pool(pool);
    let baru = GangguanBaru {
        network_id: network,
        feature_id: Some(feature),
        titik: None,
        jenis: "pipa_bocor".into(),
        deskripsi: "Bocor di sambungan".into(),
        pelapor: "Ani".into(),
    };
    let g = store.buat_gangguan(baru, "operator").await.unwrap();
    for i in 0..5 {
        let berkas = format!("{i:032x}.jpg");
        store.tambah_foto(g.id, &berkas, "image/jpeg", 1024, "operator").await.unwrap();
    }
    let keenam = store.tambah_foto(g.id, &format!("{:032x}.png", 99), "image/png", 1024, "operator").await;
    assert!(matches!(keenam, Err(StoreError::Om(OmError::Aturan(_)))), "{keenam:?}");
    let daftar = store
        .daftar_gangguan(FilterGangguan { network_id: Some(network), ..Default::default() }, 10)
        .await
        .unwrap();
    assert_eq!(daftar[0].foto.len(), 5);
}

#[ignore = "butuh database PostgreSQL"]
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn om_perawatan_selesai_menghitung_tanggal_berikutnya(pool: PgPool) {
    let (network, feature) = jaringan_dan_aset(&pool, "baik").await;
    let store = Store::from_pool(pool);
    let jadwal = store
        .tambah_jadwal(
            feature,
            JadwalBaru {
                jenis: "pembersihan_bak".into(),
                interval_hari: 90,
                tanggal_berikutnya: Some("2026-01-15".into()),
            },
            t("2026-01-01"),
            "operator",
        )
        .await
        .unwrap();
    assert_eq!(jadwal.tanggal_berikutnya, "2026-01-15");

    let selesai = store
        .catat_perawatan_selesai(jadwal.id, Some("2026-01-15"), t("2026-04-10"), "operator")
        .await
        .unwrap();
    assert_eq!(selesai.terakhir_dilakukan.as_deref(), Some("2026-01-15"));
    assert_eq!(selesai.tanggal_berikutnya, "2026-04-15");

    // Lima hari sebelum jatuh tempo: masuk jendela 30 hari dengan status segera.
    let segera = store.jatuh_tempo(network, t("2026-04-10"), 30).await.unwrap();
    assert_eq!(segera.len(), 1);
    assert_eq!(segera[0].status, "segera");
    assert_eq!(segera[0].selisih_hari, 5);

    // Lewat lima hari: tetap masuk daftar dengan status terlambat.
    let terlambat = store.jatuh_tempo(network, t("2026-04-20"), 30).await.unwrap();
    assert_eq!(terlambat[0].status, "terlambat");
    assert_eq!(terlambat[0].selisih_hari, -5);

    // Pencatatan tanggal masa depan ditolak.
    let masa_depan = store.catat_perawatan_selesai(jadwal.id, Some("2026-05-01"), t("2026-04-10"), "operator").await;
    assert!(matches!(masa_depan, Err(StoreError::Om(OmError::Aturan(_)))));
}

#[ignore = "butuh database PostgreSQL"]
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn om_rehab_mengikuti_kondisi_dan_urutan_prioritas(pool: PgPool) {
    let (network, rusak) = jaringan_dan_aset(&pool, "tidak_berfungsi").await;
    let baik: i64 = sqlx::query_scalar(
        "INSERT INTO features (network_id, feature_type, geom, kondisi) \
         VALUES ($1, 'pipa', ST_SetSRID(ST_GeomFromText('LINESTRING(107.2 -6.8, 107.21 -6.8)'), 4326), 'baik') RETURNING id",
    )
    .bind(network)
    .fetch_one(&pool)
    .await
    .unwrap();
    let ringan: i64 = sqlx::query_scalar(
        "INSERT INTO features (network_id, feature_type, geom, kondisi) \
         VALUES ($1, 'valve', ST_SetSRID(ST_MakePoint(107.3, -6.8), 4326), 'rusak_ringan') RETURNING id",
    )
    .bind(network)
    .fetch_one(&pool)
    .await
    .unwrap();
    let store = Store::from_pool(pool);

    // Aset dengan kondisi baik tidak bisa ditandai rehabilitasi.
    let tolak = store.simpan_rehab(baik, StatusRehab::Perlu, None, None, "operator").await;
    assert!(matches!(tolak, Err(StoreError::Om(OmError::Aturan(_)))), "{tolak:?}");

    // Aset rusak berat dengan prioritas 3, dan aset rusak ringan dengan prioritas 1.
    store.simpan_rehab(rusak, StatusRehab::Prioritas, Some(3), None, "operator").await.unwrap();
    store.simpan_rehab(ringan, StatusRehab::Prioritas, Some(1), None, "operator").await.unwrap();

    let daftar = store.daftar_prioritas(network).await.unwrap();
    let urut: Vec<i64> = daftar.iter().map(|b| b.feature_id).collect();
    assert_eq!(urut, vec![ringan, rusak], "prioritas 1 dulu, aset baik tidak masuk");
    assert!(!urut.contains(&baik));
}
