<div align="center">

# Gojo Lang

**Güvenmediğin Kodu Derleme.**
*(Don't compile what you don't trust.)*

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-CC_BY--NC--SA_4.0-red)]()
[![Version](https://img.shields.io/badge/version-0.1.0-orange)]()
[![Performance](https://img.shields.io/badge/performance-Native_Rust-blue)]()

*Python kadar sade, Rust kadar performanslı.*

</div>

---

## Genel Bakış (Overview)

**Gojo**, modern sistem programlama dillerinin güvenlik açıklarına tepki olarak doğmuş, **Rust/LLVM** tabanlı bir programlama dilidir. 

Geleneksel dillerin aksine, Gojo **"Runtime Aptaldır, Derleyici Zekidir"** felsefesini benimser. Güvensiz (Untrusted) bir verinin, temizlenmeden (Validate edilmeden) kritik sistemlere ulaşmasına **derleme aşamasında** engel olur.

Performans testlerinde (Fibonacci-40) **Go dilinden 2 kat, Python'dan 50 kat daha hızlı** çalıştığı kanıtlanmıştır (239ms).

## Temel Özellikler (Key Features)

* *** Sıfır Güven (Zero Trust):** Dış dünyadan (Internet, Klavye) gelen her veri `Untrusted` tipindedir. `validate` bloğu olmadan kullanılamaz.
* *** Native Hız:** Sanal Makine (VM) veya Garbage Collector (GC) kullanmaz. Kodunuz doğrudan optimize edilmiş makine koduna dönüşür.
* *** Akıllı Motor:** Fonksiyonunuz matematiksel ise (Deterministic) saf makine kodu üretir; ağ işlemi yapıyorsa (Nondeterministic) otomatik asenkron (Async) moda geçer.
* *** Zombie-Free Concurrency:** `spawn` edilen her işlem bir `scope` (kapsam) içinde yaşar. Kapsam bitince temizlik yapılır.

---

## * Hızlı Başlangıç (Quick Start)

### Kurulum (Installation)
Gojo şu an Windows (x64) sistemlerde çalışmaktadır.

1.  **Release** sekmesinden son sürümü indirin.
2.  `kurulum.bat` dosyasına sağ tıklayıp **Yönetici Olarak Çalıştır** deyin.
3.  Terminali açın ve `gojo` yazarak test edin.

### Kullanım (Usage)

Bir metin belgesi oluşturun (örn: `test.gj`) ve terminalden çalıştırın:

```bash
gojo test.gj