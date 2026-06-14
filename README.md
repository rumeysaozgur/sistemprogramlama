# aitoolgrep

`aitoolgrep`, Windows üzerinde tek bir `.exe` olarak çalışan, Rust ile yazılmış hızlı ve
yapay zeka dostu bir kod arama, değiştirme ve istatistik aracıdır.

Araç; recursive klasör gezme için `walkdir`, paralel dosya işleme için `rayon`, regex
arama için `regex`, CLI ayrıştırma için `clap`, JSON çıktısı için `serde` ve
`serde_json` kullanır.

## Ripgrep'ten Farkları

- Arama, güvenli metin değiştirme ve proje istatistiklerini tek araçta toplar.
- Her değiştirme için dosya yolu, satır numarası, eski satır ve yeni satırı raporlar.
- `--dry-run` ile diske yazmadan önce tüm değişiklikleri gösterir.
- `--backup` ile değiştirilen her dosyanın `.bak` kopyasını oluşturabilir.
- Tüm komutlar, yapay zeka araçlarının kolayca ayrıştırabileceği tek bir JSON raporu üretebilir.
- Binary ve UTF-8 olmayan dosyaları sayarak atlar.

`aitoolgrep`, ripgrep'in tüm seçeneklerini yeniden uygulamayı amaçlamaz. Ders projesinin
odağı; hızlı paralel tarama, güvenli değiştirme ve makine tarafından ayrıştırılabilir
raporlamadır.

## Varsayılan Davranış

- Aramalar varsayılan olarak büyük/küçük harfe duyarlı ve literal metin aramasıdır.
- `--ignore-case` büyük/küçük harf duyarlılığını kapatır.
- `--regex` deseni düzenli ifade olarak yorumlar.
- `.git`, `bin`, `obj`, `node_modules` ve `.vs` klasörleri atlanır.
- NUL byte içeren binary dosyalar ve geçerli UTF-8 olmayan dosyalar atlanır.
- Dosyalar paralel işlenir; sonuçlar kararlı dosya ve satır sırasıyla raporlanır.
- `replace` satır odaklı literal değiştirme yapar ve satır sonlarını korur.

## Kurulum ve Release Build

Rust araç zincirini [rustup](https://rustup.rs/) ile kurduktan sonra proje klasöründe:

```powershell
cargo build --release
```

Windows çıktı dosyası:

```text
target/release/aitoolgrep.exe
target/release/aitoolgrep-gui.exe
```

`aitoolgrep.exe` mevcut komut satırı aracıdır. `aitoolgrep-gui.exe` ise çift
tıklayarak açabileceğiniz grafik arayüzlü sürümdür.

İsterseniz `.exe` dosyalarını başka bir klasöre kopyalayabilirsiniz. Komut satırı
sürümünü terminalden kolayca kullanmak için `PATH` ortam değişkenine ekleyebilirsiniz.

## Grafik Arayüz

Grafik arayüzü açmak için:

```powershell
.\target\release\aitoolgrep-gui.exe
```

Arayüzde:

- Dosya veya klasör yolu yazılabilir ya da seçim penceresi kullanılabilir.
- Arama sekmesinde büyük/küçük harf duyarsız arama ve regex seçilebilir.
- Değiştirme sekmesi varsayılan olarak güvenli `dry-run` modunda açılır.
- Gerçek değiştirme yapılmadan önce ayrıca onay penceresi gösterilir.
- `.bak` yedek oluşturma ve JSON sonuç görünümü seçilebilir.
- İstatistik sekmesi dosya, satır, byte ve atlanan dosya sayılarını gösterir.
- Uzun işlemler arka planda çalışır; arayüz donmaz.

## Kullanım

Genel yardım:

```powershell
aitoolgrep --help
aitoolgrep search --help
aitoolgrep replace --help
aitoolgrep stats --help
```

### Search

```powershell
aitoolgrep search "LoginController" ./src
aitoolgrep search "public class" . --json
aitoolgrep search "logincontroller" ./src --ignore-case
aitoolgrep search "class\s+\w+" . --regex
aitoolgrep search "TODO|FIXME" . --regex --json
```

Her eşleşme için dosya yolu, 1 tabanlı satır numarası ve satır içeriği gösterilir.
`--case-sensitive` varsayılan davranışı açıkça belirtmek için de kullanılabilir.

### Replace

Önce güvenli ön izleme yapılması önerilir:

```powershell
aitoolgrep replace "Ogrenci" "Student" ./src --dry-run
aitoolgrep replace "Ogrenci" "Student" ./src --dry-run --json
```

Değişikliği uygulamak ve her dosya için `.bak` oluşturmak:

```powershell
aitoolgrep replace "oldName" "newName" . --backup
```

Örneğin `src/model.rs` değiştirilirse yedek `src/model.rs.bak` olur. `--dry-run`
kullanıldığında dosya ve yedek oluşturulmaz.

### Stats

```powershell
aitoolgrep stats .
aitoolgrep stats ./src --json
```

`stats`; bulunan dosya sayısını, taranan UTF-8 dosyalarını, atlanan dosyaları,
binary/UTF-8 olmayan/okunamayan dosyaları, atlanan klasörleri, toplam satırı ve
toplam taranan byte miktarını gösterir.

## Örnek JSON Çıktısı

Search:

```json
{
  "command": "search",
  "pattern": "LoginController",
  "root": "./src",
  "ignore_case": false,
  "regex": false,
  "matches": [
    {
      "path": "./src/controllers/login.rs",
      "line_number": 12,
      "line": "pub struct LoginController {"
    }
  ],
  "summary": {
    "total_files": 8,
    "scanned_files": 7,
    "skipped_files": 1,
    "binary_files": 1,
    "non_utf8_files": 0,
    "unreadable_files": 0,
    "skipped_directories": 2,
    "matches": 1
  },
  "errors": []
}
```

Replace değişiklik kaydı:

```json
{
  "path": "./src/model.rs",
  "line_number": 8,
  "old_line": "let oldName = load();",
  "new_line": "let newName = load();",
  "replacements": 1
}
```

## Testler

Projede şu entegrasyon testleri bulunur:

- Case-insensitive search ve varsayılan klasör atlama testi
- Replace `--dry-run` testi
- Replace `.bak` yedek testi
- Stats ve binary dosya atlama testi

Çalıştırmak için:

```powershell
cargo test
```

Kod formatını kontrol etmek için:

```powershell
cargo fmt --check
```

## Proje Yapısı

```text
src/
  main.rs       CLI komutları ve hata yönetimi
  bin/
    aitoolgrep-gui.rs Grafik arayüzlü ayrı Windows executable
  lib.rs        Test edilebilir kütüphane modülleri
  files.rs      Recursive keşif, hariç tutma ve UTF-8/binary kontrolü
  search.rs     Paralel arama
  replace.rs    Dry-run, backup ve paralel değiştirme
  stats.rs      Paralel proje istatistikleri
  output.rs     İnsan tarafından okunabilir ve JSON çıktıları
tests/
  integration_tests.rs
```

## Ödev Sunumunda Nasıl Anlatılır?

1. Problemi açıklayın: Kod projelerinde yalnızca arama değil, güvenli değiştirme ve
   makine tarafından okunabilir raporlama da gerekir.
2. Mimariyi anlatın: `walkdir` dosyaları keşfeder, ortak katman binary/UTF-8
   kontrolünü yapar, `rayon` dosyaları paralel işler.
3. Güvenliği gösterin: Aynı replace komutunu önce `--dry-run`, sonra `--backup`
   ile çalıştırın.
4. Yapay zeka kullanımını gösterin: `--json` çıktısında dosya, satır, eski satır,
   yeni satır ve özet alanlarını gösterin.
5. Testleri ve release çıktısını gösterin: `cargo test`, ardından
   `cargo build --release`.

Kısa sunum metni:

> aitoolgrep, Rust ile geliştirdiğim paralel bir kod arama ve güvenli metin değiştirme
> aracıdır. Walkdir ile klasörleri recursive gezer, Rayon ile dosyaları paralel işler,
> binary ve UTF-8 olmayan dosyaları atlar. Ripgrep benzeri aramaya ek olarak dry-run,
> otomatik yedek, proje istatistikleri ve yapay zeka araçlarının ayrıştırabileceği JSON
> raporları sunar. Böylece değişiklikler uygulanmadan önce satır bazında denetlenebilir.
