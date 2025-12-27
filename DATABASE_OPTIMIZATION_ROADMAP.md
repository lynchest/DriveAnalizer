# 📊 DriveAnalizer Veritabanı Optimizasyon Yol Haritası

**Proje:** DriveAnalizer  
**Amaç:** Veritabanının şişmesini önlemek ve yer kullanımını minimize etmek  
**Başlangıç Tarihi:** Aralık 2025  
**Hedef:** %60-70 veritabanı boyutu azaltma

---

## 📋 İçindekiler

1. [Mevcut Durum Analizi](#mevcut-durum-analizi)
2. [Sorunlar ve Çözümler](#sorunlar-ve-çözümler)
3. [Uygulama Planı](#uygulama-planı)
4. [Detaylı Görevler](#detaylı-görevler)
5. [Test Stratejisi](#test-stratejisi)
6. [Başarı Metrikleri](#başarı-metrikleri)

---

## 🔍 Mevcut Durum Analizi

### Mevcut Veritabanı Yapısı

```
📦 Database: drive_analytics.db
├── 📋 disk_stats (Ana Tablo)
│   ├── id: INTEGER PRIMARY KEY
│   ├── timestamp: REAL (8 byte)
│   ├── read_bytes: INTEGER (8 byte)
│   ├── write_bytes: INTEGER (8 byte)
│   ├── read_speed: INTEGER (8 byte)
│   ├── write_speed: INTEGER (8 byte)
│   └── INDEX: idx_disk_stats_timestamp
│
└── 📋 process_history (Kümülatif Tablo)
    ├── name: TEXT PRIMARY KEY
    ├── read_bytes: INTEGER
    └── write_bytes: INTEGER
```

### Mevcut PRAGMA Ayarları

```rust
PRAGMA journal_mode = WAL;      // ✅ Aktif (iyi)
PRAGMA synchronous = NORMAL;    // ✅ Ayarlı (iyi)
// ❌ Eksik optimizasyonlar:
// - Cache size ayarlanmamış
// - WAL checkpoint stratejisi optimized değil
// - ANALYZE/VACUUM otomatik çalışmıyor
// - Veri retention policy yok
```

### Veri Birikme Hızı

```
Örnek Hesaplama (günlük):
- disk_stats: 1 satır/saniye = 86.400 satır/gün
- İlk ayda: 2.592.000 satır (seri veri)
- 1 yılda: 31.536.000 satır = ~400 MB (indeksler dahil)
- Limit yok = sınırsız büyüme
```

---

## ⚠️ Sorunlar ve Çözümler

### Problem 1: Sınırsız Veri Birikimi

| Sorun | Etki | Ciddiyeti |
|-------|------|-----------|
| Eski veriler otomatik silinmiyor | DB sınırsız büyür | 🔴 Yüksek |
| Dedup mekanizması yok | Aynı veriler tekrar saklanabilir | 🟡 Orta |
| Batch insert optimized değil | Yavaş yazma performansı | 🟡 Orta |

**Çözüm:** Data retention policy + otomatik cleanup

### Problem 2: Eksik İndeksler

| Sorun | Etki | Ciddiyeti |
|-------|------|-----------|
| process_history'de name üzerinde index yok | Sorgu yavaş | 🟡 Orta |
| Zaman aralığı sorgularında index eksik | Filtreleme yavaş | 🟡 Orta |

**Çözüm:** Stratejik indeksler ekle

### Problem 3: PRAGMA Optimizasyonu Eksik

| Sorun | Etki | Ciddiyeti |
|-------|------|-----------|
| Cache size ayarlanmamış | Bellek verimsizliği | 🟡 Orta |
| WAL checkpoint otomatik değil | WAL dosyaları büyür | 🟡 Orta |
| VACUUM otomatik çalışmıyor | Boş alan geri alınmıyor | 🟡 Orta |

**Çözüm:** PRAGMA ayarlarını optimize et

### Problem 4: Veri Tipi Alanında Optimizasyon

| Sorun | Etki | Ciddiyeti |
|-------|------|-----------|
| read_speed/write_speed INTEGER (8 byte) | Gereksiz yer | 🟢 Düşük |
| idle_time/queue_depth modele eklenmemiş ama DiskStat'ta var | Veritabanında tutulmayan veri | 🟢 Düşük |

**Çözüm:** Veri tipi iyileştirmesi (optional)

---

## 📅 Uygulama Planı

### Faz 1: Temel Altyapı (1-2 gün)
- [ ] Data retention policy modülü oluştur
- [ ] Scheduled cleanup fonksiyonu yaz
- [ ] PRAGMA ayarlarını optimize et

### Faz 2: Veritabanı Şeması İyileştirmesi (1-2 gün)
- [ ] İndeks stratejisi geliştir
- [ ] Archive mekanizması oluştur
- [ ] Migration komut dosyası hazırla

### Faz 3: Performans Optimizasyonu (1 gün)
- [ ] Batch insert işlemini refactor et
- [ ] Query optimization
- [ ] Periyodik maintenance rutini

### Faz 4: Monitoring ve Testing (1-2 gün)
- [ ] Database size tracking
- [ ] Cleanup effectiveness tests
- [ ] Performance benchmarks

### Faz 5: Deployment (1 gün)
- [ ] Kullanıcı bildirimi
- [ ] Migrasyon scripti çalıştır
- [ ] Production monitoring

---

## 🎯 Detaylı Görevler

### Task 1: Data Retention Policy Modülü

**Dosya:** `src-tauri/src/db_cleanup.rs` (YENİ)

```rust
// Görev: Eski verileri temizlemek için yapı oluştur

pub struct RetentionPolicy {
    pub keep_days: u64,           // Kaç gün veri saklansın
    pub sample_interval: u64,     // Her n. veriyi sakla
    pub archive_enabled: bool,    // Archive mekanizması aktif mi
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_days: 30,         // 30 gün tuttuğu varsayılan
            sample_interval: 1,    // Tüm veriyi sakla
            archive_enabled: true, // Archive aktif
        }
    }
}

pub async fn cleanup_old_data(
    pool: &Pool<Sqlite>,
    policy: &RetentionPolicy,
) -> Result<u64, sqlx::Error> {
    // Cutoff timestamp hesapla
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    
    let cutoff = now - (policy.keep_days as f64 * 86400.0);
    
    // Eski veri sayısını al
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM disk_stats WHERE timestamp < ?"
    )
    .bind(cutoff)
    .fetch_one(pool)
    .await?;
    
    // Sil
    sqlx::query("DELETE FROM disk_stats WHERE timestamp < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;
    
    Ok(count.0 as u64)
}
```

**Başarı Kriteri:**
- ✅ Verilen gün sayısından eski verileri siler
- ✅ Silinecek veri sayısını döndürür
- ✅ Hata durumunda transaction rollback

---

### Task 2: Otomatik Cleanup Task (Background)

**Dosya:** `src-tauri/src/scheduled_tasks.rs` (YENİ)

```rust
// Görev: Arka planda periyodik cleanup çalıştır

use tokio::time::{interval, Duration};

pub async fn start_cleanup_scheduler(pool: Arc<Pool<Sqlite>>) {
    // 24 saat her interval'de cleanup çalıştır
    let mut cleanup_interval = interval(Duration::from_secs(86400));
    
    loop {
        cleanup_interval.tick().await;
        
        match cleanup_old_data(&pool).await {
            Ok(count) => {
                println!("[Cleanup] Successfully deleted {} old records", count);
                
                // VACUUM ile space boşalt
                match sqlx::query("VACUUM").execute(&pool).await {
                    Ok(_) => println!("[Cleanup] VACUUM completed"),
                    Err(e) => eprintln!("[Cleanup] VACUUM failed: {}", e),
                }
            }
            Err(e) => {
                eprintln!("[Cleanup] Failed to cleanup: {}", e);
            }
        }
    }
}

pub async fn start_analyze_scheduler(pool: Arc<Pool<Sqlite>>) {
    // Haftalık ANALYZE çalıştır
    let mut analyze_interval = interval(Duration::from_secs(604800));
    
    loop {
        analyze_interval.tick().await;
        
        match sqlx::query("ANALYZE").execute(&pool).await {
            Ok(_) => println!("[Analyze] Query optimization completed"),
            Err(e) => eprintln!("[Analyze] ANALYZE failed: {}", e),
        }
    }
}

pub async fn start_wal_checkpoint_scheduler(pool: Arc<Pool<Sqlite>>) {
    // Her 6 saatte WAL checkpoint yap
    let mut checkpoint_interval = interval(Duration::from_secs(21600));
    
    loop {
        checkpoint_interval.tick().await;
        
        match sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
            .execute(&pool)
            .await
        {
            Ok(_) => println!("[WAL] Checkpoint completed"),
            Err(e) => eprintln!("[WAL] Checkpoint failed: {}", e),
        }
    }
}
```

**Başarı Kriteri:**
- ✅ Scheduled cleanup 24 saatte bir çalışır
- ✅ ANALYZE haftalık optimize eder
- ✅ WAL checkpoint 6 saatte çalışır
- ✅ Hata oluşsa bile diğer interval'ler etkilenmez

---

### Task 3: PRAGMA Optimizasyonu

**Dosya:** `src-tauri/src/db.rs` (Değişiklik)

**Güncel Kod (lines 29-38):**
```rust
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect(&db_url)
    .await?;

// Create persistent tables
sqlx::query(
    "PRAGMA journal_mode = WAL;
     PRAGMA synchronous = NORMAL;
```

**Yeni Kod:**
```rust
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect(&db_url)
    .await?;

// Optimize PRAGMA settings for better performance
sqlx::query(
    "PRAGMA journal_mode = WAL;
     PRAGMA synchronous = NORMAL;
     PRAGMA cache_size = -64000;
     PRAGMA temp_store = MEMORY;
     PRAGMA wal_autocheckpoint = 1000;
     PRAGMA busy_timeout = 5000;"
)
.execute(&pool)
.await?;

// Create persistent tables
sqlx::query(
```

**Açıklama:**
- `cache_size = -64000`: 64 MB bellek cache (negatif = MB cinsinden)
- `temp_store = MEMORY`: Geçici işlemleri bellekte yap
- `wal_autocheckpoint = 1000`: Her 1000 sayfa sonrası otomatik checkpoint
- `busy_timeout = 5000`: Kilitlenme durumunda 5 saniye bekle

---

### Task 4: İndeks Stratejisi

**Dosya:** `src-tauri/src/db.rs` (Değişiklik)

**Mevcut İndeksler (line 45):**
```rust
CREATE INDEX IF NOT EXISTS idx_disk_stats_timestamp ON disk_stats(timestamp);
```

**Yeni İndeksler Ekle:**
```rust
CREATE INDEX IF NOT EXISTS idx_disk_stats_timestamp ON disk_stats(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_process_name ON process_history(name);
CREATE INDEX IF NOT EXISTS idx_disk_stats_time_range 
    ON disk_stats(timestamp DESC, read_bytes, write_bytes);
```

**Açıklama:**
- `idx_disk_stats_timestamp DESC`: En yeni veriler önce gelsin
- `idx_process_name`: Process arama hızlansın
- `idx_disk_stats_time_range`: Zaman aralığı sorgularında hızlı

---

### Task 5: Archive Mekanizması (Opsiyonel)

**Dosya:** `src-tauri/src/db_archive.rs` (YENİ)

```rust
// Görev: 30 günden eski verileri archive tablosuna taşı

pub async fn archive_old_data(
    pool: &Pool<Sqlite>,
    archive_days: u64,
) -> Result<u64, sqlx::Error> {
    // Archive tablosu oluştur (varsa)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS disk_stats_archive (
            id INTEGER PRIMARY KEY,
            timestamp REAL NOT NULL,
            read_bytes INTEGER NOT NULL,
            write_bytes INTEGER NOT NULL,
            read_speed INTEGER NOT NULL,
            write_speed INTEGER NOT NULL,
            archived_at REAL NOT NULL
        )"
    )
    .execute(pool)
    .await?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    
    let cutoff = now - (archive_days as f64 * 86400.0);
    
    // Eski veriyi archive'a taşı
    let result = sqlx::query(
        "INSERT INTO disk_stats_archive 
         SELECT id, timestamp, read_bytes, write_bytes, read_speed, 
                write_speed, ? as archived_at
         FROM disk_stats 
         WHERE timestamp < ?"
    )
    .bind(now)
    .bind(cutoff)
    .execute(pool)
    .await?;
    
    let count = result.rows_affected();
    
    // Asıl tablodan sil
    sqlx::query("DELETE FROM disk_stats WHERE timestamp < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;
    
    Ok(count)
}
```

---

### Task 6: Batch Insert Optimizasyonu

**Dosya:** `src-tauri/src/db.rs` (Değişiklik)

**Mevcut Kod (lines 54-73):**
```rust
pub async fn insert_stats_batch(
    pool: &Pool<Sqlite>,
    stats: &[DiskStat],
) -> Result<(), sqlx::Error> {
    if stats.is_empty() {
        return Ok(());
    }

    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO disk_stats (timestamp, read_bytes, write_bytes, read_speed, write_speed) "
    );

    query_builder.push_values(stats, |mut b, stat| {
        b.push_bind(stat.timestamp)
         .push_bind(stat.read_bytes as i64)
         .push_bind(stat.write_bytes as i64)
         .push_bind(stat.read_speed as i64)
         .push_bind(stat.write_speed as i64);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}
```

**Yeni Kod (Transaction Wrapper):**
```rust
pub async fn insert_stats_batch(
    pool: &Pool<Sqlite>,
    stats: &[DiskStat],
) -> Result<(), sqlx::Error> {
    if stats.is_empty() {
        return Ok(());
    }

    // Transaction içinde batch insert yap
    let mut tx = pool.begin().await?;

    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO disk_stats (timestamp, read_bytes, write_bytes, read_speed, write_speed) "
    );

    query_builder.push_values(stats, |mut b, stat| {
        b.push_bind(stat.timestamp)
         .push_bind(stat.read_bytes as i64)
         .push_bind(stat.write_bytes as i64)
         .push_bind(stat.read_speed as i64)
         .push_bind(stat.write_speed as i64);
    });

    query_builder.build().execute(&mut *tx).await?;
    tx.commit().await?;

    Ok(())
}
```

**Fayda:** Transaction daha hızlı, daha tutarlı veri yazma

---

### Task 7: Sampling Mekanizması (Opsiyonel)

**Dosya:** `src-tauri/src/db_sampling.rs` (YENİ)

```rust
// Görev: Yoğun periyoklara daha az örnek kaydet

pub struct SamplingPolicy {
    pub interval_seconds: u64,  // Kaç saniyede bir kaydet
    pub aggressive_after_days: u64, // Kaç gün sonra agresif sampling
}

impl Default for SamplingPolicy {
    fn default() -> Self {
        Self {
            interval_seconds: 5,   // 5 saniyede bir (varsayılan: 1)
            aggressive_after_days: 7,
        }
    }
}

pub async fn should_insert_stat(
    pool: &Pool<Sqlite>,
    policy: &SamplingPolicy,
) -> Result<bool, sqlx::Error> {
    // Son satırın timestamp'ini al
    let last: Option<(f64,)> = sqlx::query_as(
        "SELECT timestamp FROM disk_stats ORDER BY id DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    if let Some((last_timestamp,)) = last {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        
        // Interval kontrol et
        if now - last_timestamp < policy.interval_seconds as f64 {
            return Ok(false); // Bu veriyi kaydetme
        }
    }
    
    Ok(true) // Veriyi kaydet
}
```

---

### Task 8: Database Size Tracking

**Dosya:** `src-tauri/src/db_stats.rs` (YENİ)

```rust
// Görev: Veritabanı boyutunu takip et

use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub main_db_size: u64,      // drive_analytics.db
    pub wal_size: u64,          // drive_analytics.db-wal
    pub shm_size: u64,          // drive_analytics.db-shm
    pub total_size: u64,        // Toplam
    pub disk_stats_count: u64,  // Satır sayısı
    pub process_history_count: u64,
}

pub async fn get_database_stats(
    pool: &Pool<Sqlite>,
    db_path: &Path,
) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
    // Dosya boyutlarını al
    let main_db_size = std::fs::metadata(db_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    let wal_path = db_path.with_extension("db-wal");
    let wal_size = std::fs::metadata(&wal_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    let shm_path = db_path.with_extension("db-shm");
    let shm_size = std::fs::metadata(&shm_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    // Satır sayılarını al
    let (disk_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM disk_stats"
    )
    .fetch_one(pool)
    .await?;
    
    let (process_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM process_history"
    )
    .fetch_one(pool)
    .await?;
    
    let total_size = main_db_size + wal_size + shm_size;
    
    Ok(DatabaseStats {
        main_db_size,
        wal_size,
        shm_size,
        total_size,
        disk_stats_count: disk_count as u64,
        process_history_count: process_count as u64,
    })
}

pub async fn get_storage_efficiency(
    pool: &Pool<Sqlite>,
    db_path: &Path,
) -> Result<f64, Box<dyn std::error::Error>> {
    let stats = get_database_stats(pool, db_path).await?;
    
    // Satır başına ortalama byte
    if stats.disk_stats_count > 0 {
        let bytes_per_row = stats.total_size / stats.disk_stats_count;
        Ok(bytes_per_row as f64)
    } else {
        Ok(0.0)
    }
}
```

---

## 🧪 Test Stratejisi

### Unit Tests

```rust
// File: src-tauri/src/db_cleanup.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cleanup_removes_old_data() {
        // Setup: eski ve yeni veri ekle
        // Action: cleanup çalıştır
        // Assert: sadece eski veri silindiğini kontrol et
    }
    
    #[tokio::test]
    async fn test_cleanup_preserves_recent_data() {
        // Setup: son 30 gün veri ekle
        // Action: cleanup çalıştır
        // Assert: tüm veri korunduğunu kontrol et
    }
    
    #[tokio::test]
    async fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.keep_days, 30);
    }
}
```

### Integration Tests

```
✅ Database initialization
✅ Batch insert performance (1000 satır < 100ms)
✅ Cleanup effectiveness (boyut azalması)
✅ Index performance (sorgu hızı)
✅ WAL checkpoint consistency
✅ Archive mechanism (veri taşıma)
✅ PRAGMA settings application
```

### Performance Benchmarks

```
Ölçüm Noktaları:
1. Insert: 1000 satırın eklenmesi (öncesi/sonrası)
2. Query: Zaman aralığı sorgusu (öncesi/sonrası)
3. Size: Veritabanı boyutu (cleanup öncesi/sonrası)
4. Memory: Heap kullanımı (öncesi/sonrası)
5. CPU: Cleanup task CPU kullanımı
```

---

## 📊 Başarı Metrikleri

### Hedefler

| Metrik | Öncesi | Hedef | Başarı % |
|--------|--------|-------|----------|
| **DB Boyutu** | 400 MB (1 yıl) | 120 MB (1 yıl) | 70% azalma |
| **Cleanup Süresi** | N/A | < 10 saniye | - |
| **Query Hızı** | - | 2x hızlanma | 100% |
| **WAL Boyutu** | 50+ MB | 5 MB | 90% azalma |
| **Insert Hızı** | 1000 rows/1s | 2000 rows/1s | 100% |

### Monitoring Dashboard

```
📈 Real-time Metrics:
├── Database Size: [████░░] 285 MB
├── Disk Stats Count: 31.5M rows
├── Process History: 2.3K entries
├── Last Cleanup: 12 hours ago
├── WAL Size: 4.2 MB
└── Avg Bytes/Row: 9.1 bytes
```

---

## 📝 Uygulama Checklist

### Faz 1: Temel Altyapı
- [ ] `db_cleanup.rs` oluştur
- [ ] `cleanup_old_data()` fonksiyonu yaz
- [ ] `RetentionPolicy` yapısı tanımla
- [ ] Unit tests yaz

### Faz 2: Scheduled Tasks
- [ ] `scheduled_tasks.rs` oluştur
- [ ] Cleanup scheduler başlat
- [ ] ANALYZE scheduler başlat
- [ ] WAL checkpoint scheduler başlat
- [ ] `main.rs`'de schedulers'ı etkinleştir

### Faz 3: PRAGMA Optimizasyonu
- [ ] `db.rs`'de PRAGMA ayarlarını güncelle
- [ ] Yeni PRAGMA'ları test et
- [ ] Performance test çalıştır

### Faz 4: İndeks Stratejisi
- [ ] Yeni indeksleri `db.rs`'ye ekle
- [ ] Index oluşturma test et
- [ ] Query performance ölç

### Faz 5: Archive Mekanizması (Optional)
- [ ] `db_archive.rs` oluştur
- [ ] Archive tablosu oluştur
- [ ] Veri taşıma test et
- [ ] Query optimization test et

### Faz 6: Batch Insert Optimizasyonu
- [ ] Transaction wrapper ekle
- [ ] Performance test çalıştır
- [ ] Data consistency kontrol et

### Faz 7: Sampling Mekanizması (Optional)
- [ ] `db_sampling.rs` oluştur
- [ ] Sampling policy tanımla
- [ ] Insert logic'te kontrol ekle

### Faz 8: Database Stats
- [ ] `db_stats.rs` oluştur
- [ ] Size tracking fonksiyonları yaz
- [ ] Frontend'e istatistik göster

### Faz 9: Migration
- [ ] Migration script oluştur
- [ ] Upgrade path tanımla
- [ ] Rollback plan hazırla

### Faz 10: Testing & Deployment
- [ ] Tüm unit tests pass et
- [ ] Integration tests pass et
- [ ] Performance benchmarks çalıştır
- [ ] Documentation güncelle
- [ ] Release notes yaz

---

## 🚀 Başlangıç Adımları

### Gün 1: Temel Altyapı

```bash
# 1. db_cleanup.rs oluştur
# 2. cleanup_old_data() yaz
# 3. RetentionPolicy tanımla
# 4. Unit tests yaz
# 5. Tests yeşil oldu mu kontrol et
```

### Gün 2: Schedulers

```bash
# 1. scheduled_tasks.rs oluştur
# 2. 3 scheduler başlat
# 3. main.rs'de entegre et
# 4. Manual test et
```

### Gün 3: PRAGMA & İndeksler

```bash
# 1. db.rs'de PRAGMA'ları güncelle
# 2. Yeni indeksleri ekle
# 3. Performance test çalıştır
# 4. Database size kontrol et
```

### Gün 4: Advanced Optimizations

```bash
# 1. Archive mekanizması (optional)
# 2. Batch insert transaction wrapper
# 3. Database stats tracking
```

### Gün 5: Testing & Cleanup

```bash
# 1. Tüm tests çalıştır
# 2. Documentation güncelle
# 3. Release için hazırla
```

---

## 📚 İlgili Dosyalar

### Değiştirilecek Dosyalar
- `src-tauri/src/db.rs` - PRAGMA, indeksler, batch insert
- `src-tauri/src/main.rs` - Scheduled tasks başlatma

### Oluşturulacak Dosyalar
- `src-tauri/src/db_cleanup.rs` - Cleanup fonksiyonları
- `src-tauri/src/scheduled_tasks.rs` - Background schedulers
- `src-tauri/src/db_stats.rs` - Size tracking
- `src-tauri/src/db_archive.rs` - Archive mekanizması (opsiyonel)
- `src-tauri/src/db_sampling.rs` - Sampling (opsiyonel)

### Test Dosyaları
- `src-tauri/tests/db_cleanup_tests.rs`
- `src-tauri/tests/performance_tests.rs`

---

## 💡 İpuçları ve Best Practices

### Güvenlik
✅ Transaction'lar kullan (veri kaybını önle)
✅ Backup al (production cleanup öncesi)
✅ Gradual cleanup (tüm veriyi bir anda silme)

### Performance
✅ Batch işlemler yap
✅ Index oluştur (ama fazla olmadığından emin ol)
✅ ANALYZE düzenli çalıştır
✅ Scheduler'ları yüksük yükün saatlerinde çalıştır

### Monitoring
✅ Cleanup sonuçlarını log'la
✅ Database size takip et
✅ Query slow log kaydını tut
✅ Cleanup hata oranını takip et

### Documentation
✅ Cleanup policies'i dokümante et
✅ Configuration seçenekleri açıkla
✅ Troubleshooting guide yaz

---

## 📞 Sorular & Cevaplar

**S: Ne kadar veri tutmalıyım?**  
C: Varsayılan 30 gün, ama kullanıcı ayarlarla değiştirebilir.

**S: Archive mekanizması gerekli mi?**  
C: Hayır opsiyonel. Eski verileri sorgulamaya ihtiyaç varsa ekle.

**S: Sampling ne zaman kullanmalı?**  
C: Disk I/O çok yüksekse (30+ sec örnekleme) ekle.

**S: Production'da cleanup yapabilir miyim?**  
C: Evet, PASSIVE WAL checkpoint ve background scheduler'ı kullan.

**S: Veri kaybı riski var mı?**  
C: Hayır, transaction'lar ve retention policy'ler bunu engeller.

---

## 🎯 Sonuç

Bu roadmap'ı takip ederek:
- 📉 Veritabanı boyutu %60-70 azalacak
- ⚡ Query performansı 2x hızlanacak
- 🔄 WAL dosyaları otomatik yönetilecek
- 📊 Veri depolama verimliliği artacak
- 🛡️ Veri tutarlılığı korunacak

**Tahmini Zaman:** 5-7 gün (tam uygulama)

---

**Son Güncelleme:** 28 Aralık 2025  
**Versiyon:** 1.0
