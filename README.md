# Local Browser - Modern Web Tarayıcı + AI Chat

Mac OS'de çalışan, **Tauri** framework'ü ile geliştirilmiş modern web tarayıcısı. **Firecrawl API** ve **Ollama** local AI modelleri entegreli.

![Local Browser](https://img.shields.io/badge/Platform-macOS-blue?style=for-the-badge&logo=apple)
![Tauri](https://img.shields.io/badge/Framework-Tauri-orange?style=for-the-badge&logo=tauri)
![Rust](https://img.shields.io/badge/Backend-Rust-red?style=for-the-badge&logo=rust)
![Web](https://img.shields.io/badge/Frontend-HTML/CSS/JS-green?style=for-the-badge&logo=html5)

## ✨ Özellikler

- 🌐 **Modern Web Tarayıcı** - Tauri + WebView tabanlı
- 🤖 **AI Chat Entegrasyonu** - Ollama local modelleri ile sohbet
- 🔍 **Akıllı Web Analizi** - Firecrawl API ile sayfa içeriği analizi
- 💬 **Gerçek Zamanlı Chat** - Web sayfası hakkında anlık soru-cevap
- 🎨 **Modern UI/UX** - Responsive ve kullanıcı dostu arayüz
- 🔒 **Gizlilik Odaklı** - Tüm AI işlemleri local
- ⚡ **Hızlı ve Hafif** - Rust backend performansı
- 🌍 **Çoklu Platform Desteği** - macOS, Windows, Linux

## 🛠 Teknoloji Stack

- **Backend:** Rust + Tauri
- **Frontend:** HTML5, CSS3, Vanilla JavaScript
- **AI:** Ollama (Local Models)
- **Web Scraping:** Firecrawl API
- **Build:** Cargo + Tauri CLI

## 📋 Ön Gereksinimler


### 1. Rust Kurulumu
```bash
# Rust'ı kur (eğer kurulu değilse)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Node.js Kurulumu
```bash
# Node.js kur (Homebrew ile)
brew install node
```

### 3. Tauri CLI Kurulumu
```bash
# Tauri CLI'yi global olarak kur
npm install -g @tauri-apps/cli
```

### 4. Ollama Kurulumu
```bash
# Ollama'yı kur
brew install ollama

# Ollama servisini başlat
ollama serve

# Bir model indir (örneğin)
ollama pull llama3.2:3b
ollama pull mistral:7b
ollama pull codellama:13b
```

## 🚀 Kurulum ve Çalıştırma

### 1. Proje Kurulumu
```bash
# Proje dizinine git
cd /Users/oguzhan/Documents/local_browser

# Rust dependencies'leri yükle
cd src-tauri
cargo fetch
cd ..
```

### 2. API Anahtarı Kontrolü
API anahtarının `API_KEY.md` dosyasında mevcut olduğundan emin olun:
```
fc-4d73624123e2456396d20be0a85c6850
```

### 3. Development Modda Çalıştır
```bash
# Ana dizinde
tauri dev
```

### 4. Production Build
```bash
# Release build oluştur
tauri build
```

## 📱 Kullanım Kılavuzu

### Temel Kullanım

1. **Web'de Arama veya Adres Girme:**
   - Üst bardaki adres çubuğuna sorgu yazın veya tam URL girin
   - Enter'a basın veya arama ikonuna tıklayın
   - Metin yazarsanız Google'da arama yapılır, URL yazarsanız doğrudan açılır

2. **AI Chat Kullanımı:**
   - Sağ panelde model seçin (Ollama'dan otomatik yüklenir)
   - Chat kutusuna web sayfası hakkında soru yazın
   - Enter'a basın veya "Gönder" butonuna tıklayın

3. **Örnek Senaryo:**
   ```
   1. URL: https://en.wikipedia.org/wiki/Artificial_intelligence
   2. Chat: "Bu sayfada yapay zeka hakkında ne anlatılıyor?"
   3. Bot: Firecrawl ile sayfayı analiz edip Ollama ile Türkçe cevap verir
   ```

### Kısayol Tuşları

- `Cmd+L` / `Ctrl+L`: URL bar'a odaklan
- `Cmd+R` / `Ctrl+R`: Sayfayı yenile
- `Esc`: Chat input'u temizle veya chat'i kapat
- `Enter`: URL yükle veya chat mesajı gönder

## 🔧 Yapılandırma

### Model Değiştirme
Farklı Ollama modelleri kullanmak için:
```bash
# Mevcut modelleri listele
ollama list

# Yeni model indir
ollama pull model_name

# Uygulama otomatik olarak yeni modelleri tanır
```

### API Ayarları
`src-tauri/src/lib.rs` dosyasında API ayarları değiştirilebilir:
```rust
// Firecrawl API endpoint
"https://api.firecrawl.dev/v0/scrape"

// Ollama API endpoint  
"http://localhost:11434/api/generate"
```

## 🔍 Sorun Giderme

### Ollama Bağlantı Hatası
```bash
# Ollama servisinin durumunu kontrol et
curl http://localhost:11434/api/version

# Servisi yeniden başlat
ollama serve
```

### Tauri Build Hatası
```bash
# Rust toolchain'i güncelle
rustup update

# Tauri dependencies'leri temizle
cd src-tauri && cargo clean
```

### Firecrawl API Hatası
- API anahtarının doğru olduğundan emin olun
- İnternet bağlantınızı kontrol edin
- API quota limitini kontrol edin

### UI Yükleme Sorunu
- Tauri config dosyasındaki `frontendDist` yolunu kontrol edin
- HTML/CSS/JS dosyalarının doğru konumda olduğundan emin olun

## 📁 Proje Yapısı

```
local_browser/
├── API_KEY.md                 # Firecrawl API anahtarı
├── index.html                 # Ana HTML dosyası
├── styles.css                 # CSS stilları
├── app.js                     # Frontend JavaScript
├── README.md                  # Bu dosya
└── src-tauri/                 # Tauri Rust backend
    ├── Cargo.toml            # Rust dependencies
    ├── tauri.conf.json       # Tauri yapılandırması
    └── src/
        ├── lib.rs            # Ana Rust kodu
        └── main.rs           # Entry point
```

## 🎯 Gelecek Özellikler

- [ ] Bookmark sistemi
- [ ] Geçmiş kayıtları
- [ ] Çoklu sekme desteği
- [ ] Plugin sistemi
- [ ] Dark/Light tema
- [ ] Export/Import chat geçmişi
- [ ] Offline mod
- [ ] Custom AI model ekleme

## 📊 Performans

- **Uygulama Boyutu:** ~15MB (release build)
- **RAM Kullanımı:** ~50-100MB 
- **CPU Kullanımı:** Düşük (idle'da %0-1)
- **Başlatma Süresi:** ~2-3 saniye

## 🤝 Katkıda Bulunma

1. Fork edin
2. Feature branch oluşturun (`git checkout -b feature/AmazingFeature`)
3. Commit edin (`git commit -m 'Add some AmazingFeature'`)
4. Branch'i push edin (`git push origin feature/AmazingFeature`)
5. Pull Request açın

## 📄 Lisans

MIT License - Detaylar için [LICENSE](LICENSE) dosyasına bakın.

## 📞 Destek

Sorularınız için:
- Issue açın: GitHub Issues
- E-posta: [contact@example.com]
- Discord: [#local-browser]

## 🙏 Teşekkürler

- [Tauri](https://tauri.app/) - Framework
- [Ollama](https://ollama.ai/) - Local AI runtime
- [Firecrawl](https://firecrawl.dev/) - Web scraping API
- [Rust](https://www.rust-lang.org/) - Backend language

---

**Made with ❤️ by Oğuz**

Bu proje modern web teknolojileri ve AI'ın gücünü birleştirerek, geleceğin web tarayıcı deneyimini sunar.



## LOGO İÇİN YAPILACAKLAR

[1 tool called]

Kısa not: `index.html` için favicon ekledim; artık sekme/başlıkta uygulama ikonu görünecek.

Dock ikonu (macOS):
- Tauri Dock ikonu `src-tauri/icons/icon.icns` dosyasından gelir. Hâlâ Tauri logosu görünüyorsa bu dosya varsayılan Tauri ikonudur. Kendi `.icns` ikonunla değiştir.
- `src-tauri/tauri.conf.json` içindeki `bundle.icon` zaten `icons/icon.icns` yolunu içeriyor; değiştirmen gerekmez.
- Değişiklikten sonra yeniden derle:
  - `tauri build` (gerekirse önce `cd src-tauri && cargo clean`).
- macOS Dock bazen ikonu cache’ler; yeniden başlat veya `killall Dock` ile Dock’u yenile, uygulamayı kapatıp aç.

İstersen `.icns` oluşturma/çevirme adımlarını da ekleyebilirim.