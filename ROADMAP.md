# DriveAnalizer Geliştirme Yol Haritası (Roadmap)

Bu belge, `DriveAnalizer_Architecture.md` dosyasında belirtilen yüksek performanslı mimariyi hayata geçirmek için adım adım uygulanacak stratejiyi içerir.

## Faz 1: Proje Kurulumu ve Altyapı Hazırlığı

Bu aşamada, Tauri v2, Rust ve React ortamı en iyi uygulamalara (best-practices) göre kurulacaktır.

- [x] **Tauri Projesi Başlatma**
    - `npm create tauri-app@latest` komutu ile proje oluşturulacak.
    - Seçimler: `npm`, `React`, `TypeScript`.
- [x] **Rust Bağımlılıklarının Eklenmesi (`Cargo.toml`)**
    - `tokio`: Asenkron runtime (full features).
    - `sysinfo`: Sistem metrikleri için.
    - `sqlx`: SQLite veritabanı yönetimi (runtime-tokio-rustls, sqlite).
    - `serde` & `serde_json`: Veri serileştirme.
    - `tauri-plugin-sql`: (Opsiyonel) Eğer doğrudan frontend'den erişim gerekirse, ancak mimarimiz Rust üzerinden yönetimi öngörüyor.
- [x] **Frontend Bağımlılıklarının Eklenmesi (`package.json`)**
    - `uplot`: Yüksek performanslı grafik çizimi.
    - `zustand`: State yönetimi.
    - `clsx` / `tailwind-merge`: CSS sınıf yönetimi (Tailwind kullanılacaksa).

## Faz 2: Backend Çekirdeği (Rust) - Veri Toplama ve İşleme

UI'dan bağımsız olarak çalışan, sağlam bir veri toplama motoru geliştirilecektir.

- [x] **Veri Modellerinin Oluşturulması (`models.rs`)**
    - `DiskStat` struct'ı: Timestamp, Read Bytes, Write Bytes, Queue Length vb.
    - Serde `Serialize` trait'inin implementasyonu.
- [x] **Sistem İzleme Servisi (`monitor.rs`)**
    - `sysinfo` kütüphanesinin entegrasyonu.
    - Ayrı bir `tokio` task (thread) içinde çalışan sonsuz döngü (Loop).
    - **Örnekleme:** 1000ms (1 saniye) sabit aralık (`tokio::time::interval`).
- [x] **CLI Testi**
    - Tauri arayüzünü başlatmadan, sadece terminale log basarak veri toplama mantığının doğrulanması.

## Faz 3: Veritabanı Katmanı ve Performans (SQLite & SQLx)

Diski yormayan, "Batch Insert" stratejisine dayalı kalıcılık katmanı.

- [x] **Veritabanı Bağlantısı ve Kurulum (`db.rs`)**
    - `sqlx::SqlitePool` kurulumu.
    - `PRAGMA` ayarları: `journal_mode = WAL`, `synchronous = NORMAL`.
    - Uygulama başlangıcında tabloların otomatik oluşturulması (Migrations).
- [x] **Buffer Mekanizması**
    - Bellekte (RAM) tutulacak `Vec<DiskStat>` yapısı.
    - Buffer doluluk kontrolü (Örn: 60 kayıt veya 60 saniye).
- [x] **Batch Insert Implementasyonu**
    - Buffer dolduğunda tek bir Transaction içinde verilerin diske yazılması.
    - Yazma işlemi sırasında UI thread'inin bloklanmadığının teyidi.

## Faz 4: Frontend Geliştirme (React & uPlot)

Kullanıcı arayüzünün ve yüksek hızlı grafiklerin hazırlanması.

- [ ] **Layout ve Temel UI**
    - Dashboard iskeletinin oluşturulması.
    - Header, Sidebar ve Metrik Kartları (Anlık Okuma/Yazma hızları).
- [ ] **uPlot Entegrasyonu**
    - React içinde `uPlot` instance'ını yönetecek bir Wrapper bileşeni (`Chart.tsx`).
    - Canvas boyutlandırma ve responsive yapı.
    - Veri formatının `uPlot`'un beklediği dizi yapısına (`[timestamp[], read[], write[]]`) dönüştürülmesi.
- [ ] **State Yönetimi (Zustand)**
    - Anlık verilerin ve geçmiş grafik verilerinin tutulacağı store yapısı.

## Faz 5: Entegrasyon (IPC - Rust ↔ Frontend)

Backend ve Frontend'in haberleşmesi.

- [ ] **Event Emitter Kurulumu**
    - Rust tarafında toplanan verinin `app_handle.emit("io-update", payload)` ile fırlatılması.
- [ ] **Event Listener Kurulumu**
    - React tarafında `listen("io-update", ...)` ile verinin yakalanması.
    - Gelen verinin doğrudan grafiğe basılması (React render döngüsünü bypass ederek performans artışı).
- [ ] **Geçmiş Verilerin Yüklenmesi**
    - Uygulama açılışında son 1 saatlik verinin SQLite'tan çekilip grafiğe basılması için bir Tauri Command (`get_history`) yazılması.

## Faz 6: Test, Optimizasyon ve Paketleme

- [ ] **Performans Testleri**
    - Bellek sızıntısı (Memory Leak) kontrolü.
    - Uzun süreli çalışma (24+ saat) testi.
- [ ] **Hata Yönetimi**
    - Veritabanı kilitlenmesi veya dosya sistemi hatalarına karşı `Result` ve `Option` yapılarının doğru kullanımı.
- [ ] **Build ve Dağıtım**
    - `npm run tauri build` ile production çıktısı alma.
    - Windows için `.msi` veya `.exe` oluşturma.

---

**Not:** Bu yol haritası, projenin karmaşıklığını yönetilebilir parçalara bölmeyi ve her aşamada test edilebilir bir ürün ortaya koymayı hedefler.
