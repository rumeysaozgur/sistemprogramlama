# Öğretmene Kısa Sunum Metni

aitoolgrep, Rust ile geliştirdiğim paralel bir kod arama ve güvenli metin değiştirme
aracıdır. Walkdir ile klasörleri recursive gezer, Rayon ile dosyaları paralel işler,
binary ve UTF-8 olmayan dosyaları atlar. Ripgrep benzeri aramaya ek olarak dry-run,
otomatik yedek, proje istatistikleri ve yapay zeka araçlarının ayrıştırabileceği JSON
raporları sunar. Böylece değişiklikler uygulanmadan önce satır bazında denetlenebilir.

Demo sırasında önce:

```powershell
aitoolgrep search "LoginController" ./src --json
aitoolgrep replace "oldName" "newName" . --dry-run
aitoolgrep replace "oldName" "newName" . --backup
aitoolgrep stats .
```

komutlarını, ardından test ve release build sonucunu gösterebilirsiniz:

```powershell
cargo test
cargo build --release
```
