# 📘 Zet Lang Resmi Dökümantasyonu (v0.6.5)

Zet Lang'e hoş geldiniz. Bu dökümantasyon, dilin sözdizimini (syntax), temel konseptlerini, güvenlik modelini ve standart kütüphanesini içerir.

---

## 📑 İçindekiler

1. [Temel Konseptler](#1-temel-konseptler)
2. [Sözdizimi ve Değişkenler](#2-sözdizimi-ve-değişkenler)
3. [Kontrol Yapıları](#3-kontrol-yapıları)
4. [Fonksiyonlar (Saf ve Kirli)](#4-fonksiyonlar-saf-ve-kirli)
5. [Güvenlik Mimarisi (Validation)](#5-güvenlik-mimarisi-validation)
6. [Eşzamanlılık (Concurrency & Scope)](#6-eşzamanlılık-concurrency--scope)
7. [Standart Kütüphane (Stdlib)](#7-standart-kütüphane-stdlib)
8. [v0.3 Yeni Özellikler](#8-v03-yeni-özellikler)
9. [v0.4 Yeni Özellikler (Hata Yönetimi ve LSP)](#9-v04-yeni-özellikler-hata-yönetimi-ve-lsp)
10. [v0.5 Proje Sistemi ve CLI](#10-v05-proje-sistemi-ve-cli)
11. [v0.6 Git Paket Yöneticisi](#11-v06-git-paket-yöneticisi)
12. [v0.6.5 Merkezi Kayıt ve zet publish](#12-v065-merkezi-kayıt-ve-zet-publish)

---

## 1. Temel Konseptler

Zet, diğer yüksek seviyeli dillerden farklı bir zihniyete sahiptir. Yazdığınız kodu derlemeden önce şu kuralları işletir:

- **Sıfır Güven (Zero Trust):** Dış dünyadan alınan (Ağ, Dosya, Terminal) hiçbir veri doğrudan işlenemez. `validate` bloğu olmadan kullanmaya çalışmak **derleme hatası** verir.
- **Belirleyicilik (Determinism):** Ağ veya asenkron I/O işlemi yapan fonksiyonlar ile sadece CPU kullanan fonksiyonlar dil seviyesinde birbirinden ayrılır. `det` fonksiyon içinde asenkron I/O çağrısı **derleme hatası** verir. (`print`/`println` senkron olduğu için her yerde kullanılabilir.)
- **Yapısal Eşzamanlılık:** `spawn` edilen her arka plan görevi bir `scope` bloğu içinde yaşamak zorundadır. Scope dışında `spawn` kullanmak **derleme hatası** verir.

---

## 2. Sözdizimi ve Değişkenler

Zet, statik tipli ancak tip çıkarımı (type inference) yapabilen modern bir sözdizimine sahiptir.

### Değişken Tanımlama

Değişkenler `let` anahtar kelimesi ile tanımlanır.

```zet
let yas = 20
let isim = "Zet"
```

### Veri Tipleri

Şu an desteklenen temel veri tipleri şunlardır:

| Tip | Açıklama |
| --- | --- |
| `i64` | 64-bit Tamsayılar |
| `f64` | 64-bit Ondalıklı sayılar (v0.3) |
| `bool` | Mantıksal değer: `true` veya `false` (v0.3) |
| `char` | Tek karakter: `'A'`, `'z'`, `'\n'` (v0.3) |
| `u8` | 8-bit işaretsiz tamsayı (v0.3) |
| `String` | Metin dizileri |
| `(T1, T2, ...)` | Tuple (demet) — farklı tiplerin birleşimi (v0.3) |
| `Array<T>` | Aynı tipteki verilerin listesi |
| `Untrusted` | Dışarıdan gelen, henüz doğrulanmamış kirli veri |
| `Void` | Değer döndürmeyen fonksiyonların tipi |

### Diziler (Arrays)

Diziler köşeli parantezlerle tanımlanır ve indeks ile erişilir.

```zet
let sayilar = [10, 20, 30, 40]
let ilk_eleman = sayilar[0]
```

---

## 3. Kontrol Yapıları

### İf / Else İfadeleri

```zet
if yas > 18 {
    println("Giris izni verildi.")
} else {
    println("Giris reddedildi.")
}
```

### Döngüler (Loops)

Zet, aralık (range) tabanlı `for` döngülerini ve koşullu `while` döngülerini destekler.

```zet
// 0'dan 4'e kadar (4 dahil değil) döner
for i in 0..4 {
    println("Sayac: " + i)
}

// Adım (step) belirterek — 'by' anahtar kelimesi
for i in 0..10 by 2 {
    println("Cift: " + i)
}

// While döngüsü
let x = 0
while x < 5 {
    x = x + 1
}
```

---

## 4. Fonksiyonlar (Saf ve Kirli)

Zet'te fonksiyonlar, I/O (Girdi/Çıktı) yapıp yapmadıklarına göre ikiye ayrılır. Derleyici bu sayede kodunuzu en yüksek hızda optimize eder.

### Deterministic (Saf) Fonksiyonlar

Sadece RAM ve CPU kullanır. Asenkron bir işlem içermez. "Native C/Rust" hızında, hiçbir VM engeline takılmadan çalışır. **Asenkron I/O çağrısı içerirse derleme hatası verir.** `print`/`println` senkron olduğu için saf fonksiyonlarda da kullanılabilir.

```zet
det fn topla(a: i64, b: i64) -> i64 {
    println("Toplaniyor...")
    return a + b
}
```

> `det` yerine `deterministic` yazabilirsiniz — ikisi de geçerlidir.

### Nondeterministic (Kirli/I-O) Fonksiyonlar

İçerisinde Ağ isteği, konsol girdisi veya bekleme süresi barındıran fonksiyonlardır. Arka planda otomatik olarak Asenkron (Async/Await) hale getirilirler.

```zet
nondet fn veri_cek() -> Void {
    // I/O işlemleri burada yapılır
}
```

> `nondet` yerine `nondeterministic` yazabilirsiniz — ikisi de geçerlidir.

### `call` Anahtar Kelimesi

Bir I/O (Nondeterministic) işleminin sonucunu beklemek istiyorsanız `call` kelimesini kullanmalısınız. Bu, işlemi başlatan işçiyi duraklatır ancak tüm programı dondurmaz. **`call` yalnızca nondeterministic fonksiyonlar için kullanılabilir; saf fonksiyona `call` eklemek derleme hatası verir.**

```zet
let zaman = call Util.now()
let kullanici = call input("Adiniz: ")
let web_verisi = call HTTP.get("https://api.ornek.com")
```
### `print` ve `println`

Ekrana çıktı basmak için `print` (satır sonu yok) veya `println` (satır sonu var) kullanılır. Bu fonksiyonlar senkron olduğu için hem `det` hem `nondet` fonksiyonlarda kullanılabilir.

```zet
det fn hesapla(n: i64) -> i64 {
    println("Hesaplaniyor: " + n)
    return n * 2
}
```
---

## 5. Güvenlik Mimarisi (Validation)

Zet'in kalbi **Leke Analizi (Taint Analysis)** sistemidir. Dış dünyadan gelen veriler (`input`, `inputln`, `HTTP.get` vb.) `Untrusted` tipindedir. Bu veriyi standart değişkenlere atayamaz veya işlemlere sokamazsınız. **Derleyici, lekeli verinin `validate` bloğu olmadan kullanılmasını engeller.**

Bunu çözmek için `validate` bloğu kullanılmalıdır:

```zet
let kullanici_girdisi = call input("Adiniz: ")

// Derleyici bu blok olmadan islem yapmaniza izin vermez!
validate kullanici_girdisi {
    success: {
        // kullanici_girdisi burada "String" (Trusted) tipine donusur
        println("Giris yapan: " + kullanici_girdisi)
    }
}
```

---

## 6. Eşzamanlılık (Concurrency & Scope)

Arka planda aynı anda birden fazla iş yapmak (Multi-threading) Zet'te çok kolay ve güvenlidir.

### `spawn` (Ateşle ve Unut)

Bir fonksiyonu veya işlemi ana akışı durdurmadan arka planda başlatır. **`spawn` yalnızca `scope` bloğu içinde kullanılabilir; aksi takdirde derleme hatası verir.**

```zet
scope Islemler {
    spawn ag_istegi_gonder()
    spawn println("Bu yazi aninda ekrana basilir.")
}
```

### `scope` (Kapsam / Şantiye Şefi)

Zombi süreçleri engellemek için, `spawn` edilen tüm işlemler bir `scope` bloğu içinde olmak zorundadır. Scope bloğu, içindeki tüm işçiler görevini bitirmeden kapanmaz ve alt satıra geçilmez.

```zet
scope VeriIslemleri {
    // Bu iki islem ayni anda, paralel olarak baslar
    spawn HTTP.get("https://api.1.com")
    spawn HTTP.get("https://api.2.com")
}
// Kod buraya geldiginde, her iki HTTP isteginin de bittigi garanti altindadir.
```

---

## 7. Standart Kütüphane (Stdlib)

Zet v0.2 ile birlikte gelen yerleşik modüller:

### Ekrana Çıktı (print / println)

- `print(mesaj)` — Ekrana yazar (satır sonu yok). Senkron - her yerde kullanılabilir.
- `println(mesaj)` — Ekrana yazar (satır sonu var). Senkron - her yerde kullanılabilir.

### Kullanıcı Girdisi (input / inputln)

- `call input(mesaj: String) -> Untrusted` — Mesajı ekrana yazar (satır sonu yok), kullanıcıdan terminal üzerinden veri okur. Sonuç `Untrusted` tipindedir, kullanmadan önce `validate` gerekir.
- `call inputln(mesaj: String) -> Untrusted` — Mesajı ekrana yazar (satır sonu var), kullanıcıdan terminal üzerinden veri okur. Sonuç `Untrusted` tipindedir, kullanmadan önce `validate` gerekir.

### İnternet (HTTP)

- `call HTTP.get(url: String) -> Untrusted` — Belirtilen URL'ye asenkron HTTP GET isteği atar. Sonuç `Untrusted` tipindedir, kullanmadan önce `validate` gerekir.

### Araçlar (Util)

- `call Util.now() -> i64` — Sistem saatini Unix Epoch (milisaniye) cinsinden döndürür. Hız testleri için idealdir.
- `call Util.to_int(veri: String) -> i64` — Metinsel ifadeyi tam sayıya (Integer) çevirir.

### JSON İşlemleri

- `json(veri: String, anahtar: String) -> String` — Verilen JSON metninin içinden, belirtilen anahtara (key) ait değeri çıkarır.

---

## 8. v0.3 Yeni Özellikler

### 8.1 Yeni Primitif Tipler

#### `f64` — Ondalıklı Sayılar

```zet
let pi = 3.14159
let alan = pi * r * r
```

f64 tüm aritmetik operatörleri destekler: `+`, `-`, `*`, `/`, `%`.

#### `bool` — Mantıksal Değerler

```zet
let aktif = true
let pasif = false
if aktif && !pasif {
    println("Sistem aktif")
}
```

#### `char` — Tek Karakter

```zet
let harf = 'A'
let satir_sonu = '\n'
let mesaj = "Karakter: " + harf
```

Desteklenen kaçış dizileri: `'\n'`, `'\t'`, `'\\'`, `'\''`, `'\0'`.

#### `u8` — 8-bit İşaretsiz Tamsayı

```zet
det fn byte_islem(b: u8) -> u8 {
    return b
}
```

### 8.2 Yeni Operatörler

#### Modulo `%`

```zet
let kalan = 17 % 5    // 2
let cift_mi = n % 2 == 0
```

#### Mantıksal Operatörler `&&`, `||`, `!`

```zet
if yas >= 18 && vatandas {
    println("Oy kullanabilir")
}

if !aktif || askida {
    println("Hesap erisim disi")
}
```

Operatör önceliği (düşükten yükseğe): `||` → `&&` → karşılaştırma → aritmetik → `!`

#### Bitwise Operatörler `&`, `|`, `^`, `<<`, `>>`

```zet
let mask = 0xFF & deger
let bayraklar = a | b
let xor = a ^ b
let sola = 1 << 4        // 16
let saga = 256 >> 3      // 32
```

Operatör önceliği (düşükten yükseğe): `|` → `^` → `&` → `<<`/`>>`

### 8.3 Kontrol Akışı: `break` ve `continue`

`break` ve `continue` yalnızca döngü (`while`/`for`) içinde kullanılabilir. Döngü dışında kullanılırsa **derleme hatası** verir.

```zet
// İlk 5 asal sayıyı bul
let bulundu = 0
let n = 2
while bulundu < 5 {
    if asal_mi(n) {
        println(n)
        bulundu = bulundu + 1
    }
    n = n + 1
}

// Tek sayıları atla
for i in 0..20 {
    if i % 2 != 0 {
        continue
    }
    println("Cift: " + i)
}

// Koşulda çık
for i in 0..1000 {
    if i > 50 {
        break
    }
}
```

### 8.4 `const` Tanımlamaları

Sabit değerler `const` ile tanımlanır. Sonradan değiştirilemezler.

```zet
const MAX_DENEME = 3
const BASLIK = "Zet Lang"
const PI = 3
```

### 8.5 String Interpolation (Metin İçi İfade)

`${}` sözdizimi ile string içinde doğrudan değişken ve ifade kullanabilirsiniz. JavaScript'teki template literal'lara benzer.

```zet
let isim = "Dunya"
let yas = 42
println("Merhaba ${isim}, yasiniz ${yas}!")
println("${a} + ${b} = ${a + b}")
```

Interpolation, arka planda Rust'ın `format!()` makrosuna derlenir.

### 8.6 Tuple (Demet)

Farklı tiplerdeki değerleri tek bir yapıda gruplayabilirsiniz. Elemanlara `.0`, `.1`, `.2` şeklinde indeksle erişilir.

```zet
let nokta = (10, 20)
println(nokta.0)  // 10
println(nokta.1)  // 20

det fn swap(t: (i64, i64)) -> (i64, i64) {
    return (t.1, t.0)
}
```

Tuple tip sözdizimi: `(i64, String)`, `(bool, i64, f64)`.

### 8.7 Unary Operatörler

Tekil operatörler: `-` (negatif) ve `!` (mantıksal değil).

```zet
let x = -42
let y = -(a + b)
let z = !aktif
```

### 8.8 Çok Boyutlu Diziler

Dizilerin içine dizi koyarak matris benzeri yapılar oluşturabilirsiniz.

```zet
let matris = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
let ortadaki = matris[1][1]   // 5
```

### 8.9 Operatör Öncelik Tablosu (Düşükten Yükseğe)

| Öncelik | Operatör | Açıklama |
| --- | --- | --- |
| 1 | `\|\|` | Mantıksal VEYA |
| 2 | `&&` | Mantıksal VE |
| 3 | `==` `!=` `>` `<` `>=` `<=` | Karşılaştırma |
| 4 | `\|` | Bitwise VEYA |
| 5 | `^` | Bitwise XOR |
| 6 | `&` | Bitwise VE |
| 7 | `<<` `>>` | Bit kaydırma |
| 8 | `+` `-` | Toplama, Çıkarma |
| 9 | `*` `/` `%` | Çarpma, Bölme, Modulo |
| 10 | `!` `-` (unary) | Tekil operatörler |
| 11 | `()` `[]` `.N` | Gruplama, İndeks, Tuple erişimi |

---

## 9. v0.4 Yeni Özellikler (Hata Yönetimi ve LSP)

### 9.1 Hata Yönetimi (`Result` ve `T!`)

Fonksiyonların hata fırlatabileceğini belirtmek için tip sonuna `!` eklenir. `error("mesaj")` ile hata fırlatılır.

```zet
nondet fn bolme(a: i64, b: i64) -> i64! {
    if b == 0 {
        return error("Sıfıra bölme hatası!")
    }
    return a / b
}
```

### 9.2 `?` Operatörü ile Hata Aktarma (Propagation)

`?` operatörü, hata oluşursa anında fonksiyonun geri dönmesini ve hatanın bir üst katmana iletilmesini sağlar.

```zet
nondet fn islem() -> i64! {
    let sonuc = bolme(10, 0)? // Hata gelirse 'islem' de anında aynı hatayı döner
    return sonuc + 5
}
```

### 9.3 `catch` ile Hata Yakalama

`catch` operatörü, hata oluştuğunda sistemin çökmesini veya hatanın iletilmesini engelleyerek varsayılan (fallback) bir değer dönmesini sağlar.

```zet
nondet fn guvenli_islem() -> i64 {
    let sonuc = bolme(10, 0) catch 999
    return sonuc // Hata fırlatıldığı için 999 olur
}
```

### 9.4 Backend First-Class Özellikleri (Router)

Artık doğrudan dile gömülü olarak HTTP router oluşturabilirsiniz. Sadece `std.http` modülünü projenize ekleyip `@get` veya `@post` kullanmanız yeterlidir. `validate` blokları Zero-Trust modeli ile ağdan gelen datayı denetler.

```zet
import std.http

@post("/kullanici/ekle")
nondet fn kullanici_ekle(payload: Untrusted) -> String {
    validate payload {
        success: {
            let ad = json(payload, "ad")
            return "Başarıyla eklendi: " + ad
        }
        fail: {
            return "Geçersiz JSON Verisi"
        }
    }
}
```

### 9.5 Language Server Protocol (LSP)

IDE/Editor entegrasyonu için Zet Derleyicisi artık bir LSP sunucusu barındırmaktadır. `zet-compiler --lsp` komutu ile dil sunucusu modunda başlatılabilir. Dosyadaki kodları anlık olarak tarar, sözdizimi, scope hataları ve **Zero Trust (Taint) İhlallerini** anlık olarak editörünüze diagnostic olarak gönderir.

---

## 10. v0.5 Proje Sistemi ve CLI

Zet v0.5, tek bir `.zt` dosyasını çalıştırmanın yanında standart bir proje yapısı ve komut satırı iş akışı getirir. Yeni komutlar şunlardır:

- `zet new`: Yeni bir Zet projesi oluşturur.
- `zet run`: Manifestte tanımlanan projeyi derler ve çalıştırır.
- `zet build`: Projeden yerel bir çalıştırılabilir dosya üretir.

Eski `zet dosya.zt` kullanımı geriye dönük uyumlu olarak çalışmaya devam eder.

### 10.1 Yeni Proje Oluşturma

```sh
zet new merhaba
cd merhaba
zet run
```

`zet new merhaba` komutu aşağıdaki yapıyı oluşturur:

```text
merhaba/
├── zet.toml
├── .gitignore
└── src/
    └── main.zt
```

Oluşturulan `src/main.zt` dosyası çalışmaya hazır bir başlangıç programı içerir:

```zet
nondet fn main() -> Void {
    println("Merhaba, merhaba!")
}
```

Hedef klasör zaten varsa `zet new` mevcut dosyaların üzerine yazmaz ve hata verir. Proje adında harf, rakam, `-` ve `_` karakterleri kullanılabilir.

### 10.2 `zet.toml` Proje Manifesti

Her v0.5 projesinin kökünde bir `zet.toml` manifesti bulunur:

```toml
[package]
name = "merhaba"
version = "0.1.0"
entry = "src/main.zt"
```

| Alan | Zorunlu | Açıklama |
| --- | :---: | --- |
| `name` | Evet | Paket ve üretilen çalıştırılabilir dosyanın adı. |
| `version` | Hayır | Proje sürümü. Belirtilmezse `0.1.0` kullanılır. |
| `entry` | Hayır | Ana Zet kaynak dosyası. Belirtilmezse `src/main.zt` kullanılır. |

`zet run` ve `zet build`, geçerli klasörden üst klasörlere doğru `zet.toml` arar. Bu nedenle komutlar proje içindeki bir alt klasörden de çalıştırılabilir.

### 10.3 Projeyi Çalıştırma

Manifestteki giriş dosyasını çalıştırmak için proje içinde şu komut yeterlidir:

```sh
zet run
```

Belirli bir kaynak dosyası açıkça da seçilebilir:

```sh
zet run src/main.zt
```

Program argümanları `--` ayırıcısından sonra iletilir:

```sh
zet run -- birinci ikinci
zet run src/main.zt -- birinci ikinci
```

### 10.4 Yerel Çalıştırılabilir Dosya Üretme

```sh
zet build
```

Derleme başarılı olduğunda çıktı proje içindeki `.zet/bin/` dizinine yazılır:

```text
.zet/bin/merhaba       # Linux ve macOS
.zet/bin/merhaba.exe   # Windows
```

Belirli bir tek dosya için de build alınabilir:

```sh
zet build program.zt
```

### 10.5 İzole `.zet` Çalışma Dizini

v0.4.5 ve önceki sürümlerde üretilen Rust kaynağı paket runtime dizinine yazılıyordu. v0.5 ile her proje kendi çalışma alanını kullanır:

```text
.zet/
├── runtime/    # Üretilen Rust uygulaması ve Cargo manifesti
├── target/     # Projeye özel Cargo derleme önbelleği
└── bin/        # zet build çıktıları
```

Bu yapı farklı projelerin üretilen kaynaklarının ve derleme çıktılarının birbiriyle çakışmasını engeller. `.zet/` üretilen bir dizindir; kaynak kontrolüne eklenmemelidir. `zet new` tarafından oluşturulan `.gitignore` bunu otomatik olarak dışlar.

İlk `zet run` veya `zet build` çağrısı Rust bağımlılıklarını derlediği için daha uzun sürebilir. Aynı projedeki sonraki çağrılar `.zet/target` önbelleğini yeniden kullanır.

### 10.6 Tek Dosyalı Kullanım

Mevcut programlar proje oluşturmadan çalıştırılabilir:

```sh
zet program.zt
zet run program.zt
zet build program.zt
```

Bu kullanımda `.zet/` çalışma dizini kaynak dosyanın bulunduğu klasörde oluşturulur. Eski `zet program.zt arguman` biçimi program argümanlarını doğrudan iletmeye devam eder.

### 10.7 Diğer Komutlar

```sh
zet --version   # Derleyici sürümünü gösterir
zet --help      # Komut yardımını gösterir
zet --lsp       # Dil sunucusunu başlatır
```

### 10.8 v0.4.5'ten Geçiş

Tek dosyalı kod için değişiklik gerekmez. Bir v0.5 projesine geçmek için kaynak dosyanızı `src/main.zt` konumuna taşıyıp proje köküne aşağıdaki manifesti ekleyebilirsiniz:

```toml
[package]
name = "uygulamam"
version = "0.1.0"
entry = "src/main.zt"
```

Ardından `zet run` ve `zet build` komutları kullanılabilir. Windows, Linux ve macOS başlatıcıları çalışma klasörünü değiştirmeden komutları derleyiciye ilettiği için proje keşfi tüm desteklenen platformlarda aynı şekilde çalışır.

---

## 11. v0.6 Git Paket Yöneticisi

Zet v0.6, merkezi bir kayıt sunucusuna ihtiyaç duymadan Git depolarını proje bağımlılığı olarak kullanabilir. Doğrudan ve geçişli bağımlılıklar SemVer etiketlerinden çözülür; seçilen Git commit'i ile paket içeriğinin SHA-256 özeti `zet.lock` dosyasına yazılır.

Paket komutları için sistemde `git` komutunun `PATH` üzerinde bulunması gerekir. Kaynaktan derleme ve Zet uygulaması üretme gereksinimi olarak Rust stable toolchain kullanılmaya devam eder.

### 11.1 Paket Deposu Biçimi

Bir Git deposunun Zet paketi olabilmesi için depo kökünde `zet.toml` bulunmalıdır. `entry`, paketin dışarı açılan Zet kaynak dosyasını belirtir:

```toml
[package]
name = "ornek_math"
version = "1.2.0"
entry = "src/lib.zt"

[dependencies]
```

Paket adı import sözdizimiyle uyumlu olmalıdır: harf veya `_` ile başlamalı; devamında yalnızca harf, rakam ve `_` kullanılmalıdır. `entry` paket deposu içinde kalan göreli bir yol olmalıdır; mutlak yollar ve `..` ile üst dizine çıkış reddedilir.

Sürümler `v1.2.0` veya `1.2.0` biçimindeki Git etiketleriyle yayımlanır. Etiketteki SemVer değeri ile manifestteki `[package].version` aynı olmalıdır. Yeniden üretilebilir çözümleme için SemVer etiketi bulunmayan depolar paket olarak kurulmaz.

### 11.2 Paket Ekleme

v0.6.5 merkezi kaydındaki paketler doğrudan adlarıyla eklenebilir:

```sh
zet add ornek_math
zet add ornek_math@^1.2
```

GitHub kısaltmasıyla en yeni sürümü eklemek için:

```sh
zet add sahip/ornek-math
```

Belirli bir sürüm veya SemVer koşulu da verilebilir:

```sh
zet add sahip/ornek-math@1.2.0
zet add sahip/ornek-math@^1.2
zet add "sahip/ornek-math@>=1.2, <2.0"
```

HTTPS gibi `://` içeren tam Git URL'leri de desteklenir:

```sh
zet add https://git.example.com/ekip/ornek-math.git@1.2.0
```

`zet add`, uygun en yüksek etiketi seçer, paketin kendi `zet.toml` dosyasından adını doğrular ve çözülmüş tam sürümü projenin manifestine ekler:

```toml
[dependencies]
ornek_math = { git = "https://github.com/sahip/ornek-math.git", version = "1.2.0" }
```

Merkezi kayıt üzerinden ada göre eklenen bağımlılıklarda kaynak ayrıca işaretlenir:

```toml
ornek_math = { git = "https://github.com/sahip/ornek-math.git", version = "1.2.0", registry = "zet" }
```

Bu işaret, lock dosyası yeniden üretildiğinde veya paket güncellendiğinde yalnızca registry tarafından onaylanmış sürüm, commit ve checksum kayıtlarının kullanılmasını zorunlu kılar.

### 11.3 Kurma, Güncelleme ve Kaldırma

```sh
zet install             # Manifest ve mevcut kilide göre tüm paketleri kurar
zet update              # Tüm doğrudan paketleri en yeni SemVer etiketine taşır
zet update ornek_math   # Yalnızca seçilen doğrudan paketi günceller
zet remove ornek_math   # Bağımlılığı ve artık kullanılmayan geçişli paketleri kaldırır
```

`zet install`, geçerli bir kilit kaydı varsa aynı commit'i yeniden kullanır. Kilit yoksa manifestteki sürüm koşullarını çözüp yeni bir kilit üretir. Birden fazla bağımlılık aynı paket için uyumsuz Git deposu veya sürüm istiyorsa kurulum açık bir bağımlılık çakışmasıyla durur.

`zet update`, güncellenecek doğrudan bağımlılık için depodaki en yeni SemVer etiketini seçer ve `zet.toml` ile `zet.lock` dosyalarını birlikte yeniler. Büyük sürüm yükseltmeleri de seçilebildiği için değişiklikler kaynak kontrolüne alınmadan önce uygulama yeniden derlenmelidir.

### 11.4 `zet.lock` ve Bütünlük

Üretilen `zet.lock` dosyası doğrudan ve geçişli her paket için aşağıdaki bilgileri tutar:

```toml
lock_version = 1

[[package]]
name = "ornek_math"
git = "https://github.com/sahip/ornek-math.git"
requirement = "1.2.0"
version = "1.2.0"
commit = "0123456789abcdef0123456789abcdef01234567"
checksum = "sha256:..."
```

- `commit`, hareket ettirilemeyen kurulum kimliğidir.
- `checksum`, checkout içeriğinin deterministik SHA-256 özetidir.
- Kilit sürümü, yinelenen paket kayıtları, commit ve checksum kurulum sırasında doğrulanır.
- Sembolik bağlantı içeren paketler güvenli ve platformlar arası aynı içerik garantisi verilemediği için reddedilir.

Uygulama depolarında `zet.toml` ve `zet.lock` kaynak kontrolüne eklenmeli, üretilen `.zet/` dizini eklenmemelidir.

### 11.5 Önbellek ve Proje Checkout'u

Git depolarının bare mirror kopyaları makine genelinde ortak bir önbellekte tutulur:

| Platform | Varsayılan önbellek |
| --- | --- |
| Windows | `%LOCALAPPDATA%\\Zet\\cache` |
| Linux/macOS | `$XDG_CACHE_HOME/zet` veya `$HOME/.cache/zet` |

`ZET_CACHE_DIR` ortam değişkeni bu konumu değiştirebilir. Çözülmüş kaynaklar her proje için `.zet/packages/<paket-adı>/` altında checkout edilir. Böylece ağ ve Git nesneleri projeler arasında paylaşılırken derlemede kullanılan paket ağacı projeye özel kalır.

Kilitli commit ortak önbellekte mevcutsa `zet install` onu ağdan yeniden çözmeden kullanabilir. `zet add` ve `zet update` ise yeni etiketleri görebilmek için uzak depoyu günceller.

### 11.6 Paket Importları

Paketin giriş dosyasını import etmek için manifestteki paket adı kullanılır:

```zet
import ornek_math

nondet fn main() -> Void {
    println(ornek_math::topla(20, 22))
}
```

Paket giriş dosyasının yanındaki bir modül `import ornek_math.istatistik` biçiminde içe aktarılabilir. Paketlerin kendi `[dependencies]` bölümleri de çözülür; geçişli paketler aynı import haritasına katılır.

v0.6, Git tabanlı bağımlılık yönetiminin ilk kararlı temelidir. v0.6.5 merkezi paket keşfi, ad sahipliği ve doğrulanmış yayın akışını bu temel üzerine ekler.

---

## 12. v0.6.5 Merkezi Kayıt ve `zet publish`

Zet Registry, paket adlarını doğrulanmış GitHub depolarına bağlayan merkezi ve kaynak kontrolünde tutulan bir indekstir. Kayıt bir binary arşivi barındırmaz. Kurulum yine paketin Git deposundaki SemVer etiketi, değişmez commit ve SHA-256 bütünlük denetimi üzerinden yapılır.

Bu ayrım sayesinde merkezi indeks paket keşfi ve ad sahipliği sağlar; `zet.lock` ise belirli bir uygulamanın tam olarak hangi içeriği kullandığını sabitler.

### 12.1 Paket Arama

Tüm kayıtları listelemek veya ad/açıklama içinde aramak için:

```sh
zet search
zet search json
```

Arama sonucu paket adı, en yeni sürüm, kısa açıklama, Git deposu ve kayıt sahibini gösterir. Kayıtlı paket ada göre eklenebilir:

```sh
zet add ornek_json@^1.0
```

`sahip/depo@surum` ve tam Git URL'si biçimleri geriye dönük uyumlu olarak çalışmaya devam eder. Bir paket merkezi kayıtta bulunmasa bile doğrudan Git deposundan kurulabilir.

### 12.2 Yayımlanabilir Paket Manifesti

Yayımlanacak deponun kökündeki `zet.toml` en az aşağıdaki alanları içermelidir:

```toml
[package]
name = "ornek_json"
version = "1.0.0"
description = "Zet için küçük bir JSON yardımcı paketi"
entry = "src/lib.zt"

[dependencies]
```

- `name`, merkezi kayıtta sahiplenilecek addır ve import kurallarına uymalıdır.
- `version`, geçerli SemVer olmalıdır.
- `description`, en fazla 160 yazdırılabilir karakter olabilir.
- `entry`, depo içinde kalan, Git tarafından izlenen göreli bir dosya olmalıdır.
- `zet.toml`, entry ve paketin diğer bütün dosyaları yayın öncesinde commit edilmelidir.
- Çalışma ağacı temiz olmalı ve `zet.toml` Git deposunun kökünde bulunmalıdır.
- Checkout en fazla 2.000 dosya ve toplam 50 MiB içerik barındırabilir.

v0.6.5 kayıt akışı kişisel GitHub depolarını kabul eder. GitHub issue isteğini açan kullanıcı ile `https://github.com/<kullanıcı>/<depo>` adresindeki depo sahibi aynı olmalıdır. Organizasyon sahipliği ve yetki devri sonraki sürümlere bırakılmıştır.

### 12.3 GitHub Kimlik Doğrulaması

`zet publish`, kayıt isteğini GitHub API üzerinden açar. Önerilen yöntem GitHub CLI ile bir kez oturum açmaktır:

```sh
gh auth login
gh auth status
```

GitHub CLI kullanılmıyorsa token ortam değişkeniyle verilebilir:

```sh
export ZET_REGISTRY_TOKEN="github_token"
```

Windows PowerShell:

```powershell
$env:ZET_REGISTRY_TOKEN = "github_token"
```

Token yalnızca GitHub'a kayıt issue'su açmak için kullanılır; `zet.lock`, manifest veya registry indeksine yazılmaz. Öncelik sırası `ZET_REGISTRY_TOKEN`, `GITHUB_TOKEN`, ardından `gh auth token` çıktısıdır.

### 12.4 Yayını Önceden Doğrulama

Hiçbir etiket veya ağ yazımı yapmadan kontrolleri çalıştırmak için:

```sh
zet publish --dry-run
```

Başarılı dry-run şu bilgileri gösterir:

- Paket adı ve SemVer sürümü
- Normalize edilmiş GitHub origin URL'si
- Yayınlanacak HEAD commit'i
- Kullanılacak `vX.Y.Z` veya mevcut `X.Y.Z` etiketi

Dry-run etiketi oluşturmaz, push yapmaz ve kayıt isteği açmaz.

### 12.5 Paketi Yayımlama

```sh
zet publish
```

Komut sırasıyla şu işlemleri yapar:

1. Manifesti, entry dosyasını, Git kökünü ve temiz çalışma ağacını doğrular.
2. Aynı sürüm etiketi varsa HEAD commit'ini gösterdiğini denetler.
3. Etiket yoksa açıklamalı `v<version>` etiketi oluşturur.
4. Etiketi `origin` deposuna push eder.
5. Merkezi kayıt deposunda makinece okunabilir bir GitHub issue isteği açar.

Etiket başarıyla push edildikten sonra issue oluşturma ağ veya kimlik doğrulama nedeniyle başarısız olursa aynı komut yeniden çalıştırılabilir. Var olan ve doğru commit'i gösteren etiket yeniden kullanılacaktır.

### 12.6 Sunucu Tarafı Onay Denetimleri

Kayıt issue'su açıldığında GitHub Actions güvenilen ana daldaki doğrulayıcıyı çalıştırır. Issue içindeki kod çalıştırılmaz. Doğrulayıcı:

- İstek sahibi ile kişisel GitHub depo sahibini karşılaştırır.
- Depoyu bağımsız olarak klonlar ve bildirilen commit'i checkout eder.
- `vX.Y.Z` veya `X.Y.Z` etiketinin aynı commit'i gösterdiğini denetler.
- `zet.toml` adını, sürümünü, açıklamasını ve güvenli entry yolunu doğrular.
- Sembolik bağlantıları reddeder.
- Paket içeriğinin platformdan bağımsız SHA-256 özetini üretir.
- Dosya sayısı ile toplam checkout boyutu sınırlarını uygular.
- İlk yayında paket adını GitHub kullanıcısına tahsis eder.
- Sonraki yayınlarda sahip ve Git deposunun değişmediğini doğrular.
- Aynı sürümün farklı commit veya checksum ile değiştirilmesini reddeder.

Onaylanan istek `registry/index.json` dosyasına commit edilir ve issue otomatik kapatılır. Bundan sonra paket `zet search` ve ada göre `zet add` ile kullanılabilir. Reddedilen istek hata nedeniyle birlikte issue üzerinde bildirilir.

### 12.7 Özel Kayıt Ayarları

Varsayılan merkezi kayıt Zet Lang kaynak deposundaki indekstir. Uyumlu bir özel kayıt veya yerel geliştirme senaryosu için:

| Ortam değişkeni | Amaç |
| --- | --- |
| `ZET_REGISTRY_URL` | Okunacak uyumlu registry JSON adresini değiştirir. |
| `ZET_REGISTRY_FILE` | Ağ yerine yerel registry JSON dosyası kullanır. |
| `ZET_REGISTRY_ISSUES_API` | Yayın isteğinin gönderileceği uyumlu GitHub Issues API adresini değiştirir. |
| `ZET_REGISTRY_TOKEN` | Yayın API kimlik doğrulama token'ını sağlar. |

Registry şema sürümü v0.6.5 için `1` değeridir. Bilinmeyen şema sürümleri, geçersiz paket adları ve Git deposu eksik kayıtlar istemci tarafından reddedilir.
