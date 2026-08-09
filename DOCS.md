# 📘 Zet Lang Resmi Dökümantasyonu (v0.5)

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
