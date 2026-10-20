# Benchmark Penyedia AI Immersion

Tanggal pengujian: 20 Oktober 2026

Tujuan: memilih model gratis yang paling akurat, cepat, dan cocok untuk fitur AI Tutor
(Ctrl + Klik) di Limusic. Prompt yang diuji adalah prompt asli proyek:

- `word_explanation` — menjelaskan nuansa kata dalam konteks.
- `learning_dictionary` — entri kamus Markdown lengkap dalam Bahasa Indonesia.
- `sentence_breakdown` — memecah kalimat menjadi chunk gramatikal.
- Nuansa `subtle` dan kata hukum `charged` dipakai sebagai uji kualitas yang sulit.

## Ringkasan hasil

Latency ditulis sebagai **TTFT / total** dalam detik, diukur dari streaming SSE.
TTFT adalah waktu sampai token pertama terlihat.

### OpenRouter

| Model | Kata `run` | Kamus `charged` | Nuansa `subtle` | Breakdown |
|---|---:|---:|---:|---:|
| `dots-studio/dots-3-note-preview:free` | 1.83 / 3.39 | 0.75 / 9.28 | 1.01 / 3.24 | 0.47 / 4.13 |
| `nvidia/nemotron-3.5-lightning:free` | 3.17 / 3.90 | 1.28 / 6.38 | 1.02 / 1.86 | 1.81 / 4.50 |

`dots` menjadi pilihan utama karena nuansa dan struktur kamusnya paling konsisten.
Kekurangan yang terlihat: satu typo pada breakdown (`suggestions` seharusnya
`suggests`) dan beberapa istilah hukum yang kurang tepat. Model gratis OpenRouter
memiliki batas harian; jika terkena `429`, Limusic langsung melanjutkan ke provider
berikutnya.

Model `minimax` tidak dipakai karena output kamusnya pernah rusak (teks tercampur
simbol/karakter aneh). `laguna`, `gemma`, dan beberapa model lain sering `429`.

### NVIDIA NIM (langsung)

| Model | Kata `run` | Kamus `charged` | Nuansa `subtle` | Breakdown |
|---|---:|---:|---:|---:|
| `nvidia/nemotron-3.5-lightning-30b-a3b` | 0.61 / 1.67 | 0.80 / 6.85 | 0.69 / 2.09 | 0.71 / 2.51 |

Model tercepat secara keseluruhan, cocok sebagai fallback pertama ketika OpenRouter
sibuk. Kualitas Bahasa Indonesianya sedikit di bawah `dots`: pernah muncul
`mengakhili`, `ditetik`, dan pilihan kata yang kurang natural. Endpoint kadang
mengembalikan `403 Authorization failed` atau `503 ResourceExhausted`, jadi cooldown
dan fallback tetap diperlukan.

### Cloudflare Workers AI

| Model | Kamus `charged` | Nuansa `subtle` | Breakdown | Catatan |
|---|---:|---:|---:|---|
| `@cf/qwen/qwen3-30b-a3b-fp8` | 0.90 / 5.05 | 0.79 / 1.58 | 0.69 / 1.67 | Pilihan utama Cloudflare |
| `@cf/aisingapore/gemma-sea-lion-v4-27b-it` | 0.60 / 8.61 | 0.40 / 2.22 | 0.44 / 4.51 | Paling natural, alternate kualitas |
| `@cf/meta/llama-3.3-70b-instruct-fp8-fast` | 0.74 / 7.00 | 0.46 / 2.79 | 0.50 / 6.90 | Backup seimbang |
| `@cf/qwen/qwen3.8-27b` | 0.54 / 10.95 | 0.56 / 2.11 | 0.54 / 4.93 | Detail, tetapi lambat/verbosa |
| `@cf/google/gemma-4-26b-a4b-it` | 12.36 / 19.53 | 0.97 / 2.59 | 0.80 / 1.66 | Kamus terlalu lambat |
| `@cf/mistralai/mistral-small-3.1-24b-instruct` | 0.46 / 11.75 | — | — | Pernah salah arti (`diisukan`) |
| `@cf/nvidia/nemotron-3-120b-a12b` | 9.06 / 13.50 | — | — | TTFT tinggi, ada kosakata Malaysia |
| `@cf/meta/llama-3.2-3b-instruct` | 0.51 / 2.27 | — | — | Cepat, tetapi akurasi rendah |

Keputusan Cloudflare:

- **Default:** `@cf/qwen/qwen3-30b-a3b-fp8` karena kombinasi terbaik antara akurasi
  dan kecepatan. Kamus selesai sekitar 5 detik dan token pertama muncul di bawah 1 detik.
- **Fallback kualitas:** `@cf/aisingapore/gemma-sea-lion-v4-27b-it`. Terjemahan paling
  natural, tetapi kamus sekitar 8,6 detik.
- **Backup:** `@cf/meta/llama-3.3-70b-instruct-fp8-fast`.

Catatan teknis Cloudflare:

- Qwen 3 dan Gemma 4 harus dijalankan dengan `chat_template_kwargs.enable_thinking=false`.
  Tanpa itu, seluruh budget token habis untuk reasoning dan `content` kosong.
- Stream Cloudflare dapat mengirim angka sebagai JSON number, misalnya
  `{"content": 28}`. Parser harus mengubahnya menjadi teks; jika tidak, kalimat seperti
  “28 green-card holders” kehilangan angkanya.
- Model `@cf/zai-org/glm-5.3-flash` dan `@cf/deepseek-ai/deepseek-v4-flash-0731`
  mengembalikan `403` karena tidak tersedia di Workers Free plan. Model seperti ini
  di-cooldown per model, bukan mematikan seluruh provider.

## Keputusan akhir

Urutan fallback yang dipakai:

1. **OpenRouter** — `dots-studio/dots-3-note-preview:free`
2. **NVIDIA NIM** — `nvidia/nemotron-3.5-lightning-30b-a3b`
3. **Cloudflare Workers AI** — `@cf/qwen/qwen3-30b-a3b-fp8`
4. Cloudflare Sea Lion → Llama 3.3 70B
5. Model gratis OpenRouter lain → model dinamis dari katalog OpenRouter

Semua provider bisa dinonaktifkan dari Pengaturan. Provider yang nonaktif atau belum
punya token dilewati. Cache SQLite diperiksa sebelum kredensial, sehingga jawaban lama
tetap bisa dibuka tanpa koneksi. Token tidak pernah dikirim kembali ke frontend setelah
disimpan; UI hanya menampilkan status “Tersimpan”.
