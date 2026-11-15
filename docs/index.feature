Feature: Index: every picture and video you have, searchable in minutes (0.1.5)
  Shane, 11-14: "the big thing I need you to ship in 0.1.5 is the fastest fucking indexing tool for videos and images
  on the OS and on your phone/photos/icloud ... a plugin that is able to do this for google drive, apple photos and
  icloud and locally or on a local drive of course. if free that index just stays local. if pro or team we can factor
  it into their plan. ITs vital that a wow moment is just how fast we can index tons of content, embed it all and
  manage those embeddings across search." 0.1.5 carries this and the 0.1.4 feed fixes; Character SDK moves to 0.1.6.

  This builds on Libraries (docs/libraries.feature): its libraries, sources, local index, cloud index (turbopuffer
  through raea-api) and fused search stay. What changes is the first pass. Today a file is hashed, cut into shots and
  embedded by EmbeddingGemma 2 on the CPU before it can be found: LB1 took 316 s for 25 files, 12.6 s a file. A
  photo library of 50,000 would take a week. EmbeddingGemma 2's picture tower is a 16-layer ViT over up to 2,520
  patches, about 0.9 TFLOP a picture, so no runtime makes it the first pass.

  So indexing has two passes:
    - the fast pass: every picture, and a few frames of every video, through SigLIP 2 B/16-256 on the Neural Engine,
      from the small copy the file or Photos already holds, searchable by words through SigLIP 2's text tower. Minutes
      for a whole library.
    - the deep pass (Libraries as it is): shots, sound, transcripts and EmbeddingGemma 2, in the background, after
      the fast pass, never on battery. It adds what's said and heard, and better ranking for video.

  Measured 11-14 on this Mac (M5 Pro), with nothing else changed (ixbench in the scratchpad, 600 real files, mostly
  big PNGs with no embedded preview, the worst case for decoding):
    | step                                           | per second | note                                        |
    | ImageIO thumbnail decode to 256 px, all cores  | 383-399    | JPEG and HEIC with a preview are faster     |
    | SigLIP 2 B/16-256, Core ML, CPU + Neural Engine | 194-200    | batch 32 or 64; cosine 0.9985 to fp32 (10-16) |
    | the same on Core ML's GPU                      | 297        | wrong vectors on this Mac (coreml-gpu-siglip) |
    | EmbeddingGemma 2, q8 on CPU (LB1)              | 0.08 files | 12.6 s a file, shots and sound included     |

  Rule: The fast pass

    Scenario: IX1. A picture is found by words seconds after raea sees it
      Given a source with pictures (a folder, a drive, Photos, iCloud Drive, Google Drive)
      When the fast pass runs
      Then each picture is listed (its place, size, dates, dimensions), decoded to 256 px from its embedded preview
        when it has one (ImageIO; PhotoKit's own derivative for Photos), and embedded by SigLIP 2 B/16-256 on Core ML's
        CPU and Neural Engine, 32 at a time, while the next batch decodes
      And its vector (768 f16, normalised) and a 256 px JPEG thumbnail are written as soon as its batch is done, so
        search finds the first pictures while the rest are still going
      And an item is keyed by where it lives (a path, a Photos asset, a Drive file id) with its size and dates; its
        sha256 comes with the deep pass, or when the original is read anyway. The fast pass never reads a whole
        original only to hash it
      And sustained, it runs at 150 pictures a second or better on an M-series Mac with nothing else heavy running
        (bench/index), and says how fast it went

    Scenario: IX2. A video's first pass is a few frames
      Then a video gets its poster and one frame every 4 s, at most 16, from the nearest sync frames (AVFoundation,
        VideoToolbox's hardware decoder), each a row with its time, so "the dog jumps in the pool" finds the moment
      And a video is listed with its length and dimensions before its frames are read

    Scenario: IX3. Search reads both passes
      Then a query's words are embedded by SigLIP 2's text tower (ONNX on CPU, one query in milliseconds) and scored
        against every fast-pass row (brute force over f16 rows in Rust, fast enough to a million rows), and by
        EmbeddingGemma 2 against the deep pass's rows as now
      And the lists are fused by site/api src/library/fuse.ts with the fast list as one more list of its own weight and
        MARGIN, measured on LB1's queries plus a photo set before the weights are set; a model's vectors are never
        scored against another model's
      And a picture or a frame can be the query ("more like this"), against the fast rows
      And results show the thumbnails, grouped per file, with the moment for a video

    Scenario: IX4. The deep pass follows
      Then once a source's fast pass is done, Libraries' indexFile runs on its videos, sounds and words at low priority,
        through antsy, never on battery, its own files before claimed jobs (LB6b)
      And a picture needs no deep pass to be found; its EmbeddingGemma 2 row comes when the deep pass reaches it

  Rule: Sources

    Scenario: IX5. This Mac's folders and drives
      Then a folder or a drive is listed fast (getattrlistbulk through fts, skipping packages, caches and raea's own
        media), watched with FSEvents, and a drive's mount and unmount set it online or offline (Libraries' volume
        sources); an offline drive stays searchable from its thumbnails

    Scenario: IX6. Apple Photos, and so iCloud Photos and the iPhone's camera roll
      Then turning Photos on asks macOS once (raea.app's NSPhotoLibraryUsageDescription; read-only, never a change to
        the library)
      And every asset is fetched from PhotoKit at once (PHAsset, newest first), each picture's 256 px image requested
        from PHImageManager (its local derivative; one only in iCloud is fetched at that size when "include photos only
        in iCloud" is on, by default), a video's frames from its local AVAsset, else its poster
      And an asset is keyed by its local identifier; its dates, place, favourite and media subtypes (screenshot,
        selfie, live photo, slo-mo) become filters
      And photos taken on the iPhone arrive through iCloud Photos and are indexed as they land (PHPhotoLibrary's change
        observer)

    Scenario: IX7. iCloud Drive
      Then iCloud Drive is a folder source (~/Library/Mobile Documents/com~apple~CloudDocs); a file kept only in iCloud
        is indexed from its Quick Look thumbnail (QLThumbnailGenerator) without downloading it, marked "in iCloud", and
        downloaded only when opened
      Status 11-15: until the Quick Look path, a file kept only in iCloud (SF_DATALESS) is left out and counted
        (walk.rs cloud_only; Progress.cloud, and the CLI says "N only in iCloud, left out"). Reading one downloads its
        original: on this Mac all 3,849 iCloud Drive files were cloud-only, 2,245 of them pictures and videos, and
        switching iCloud Drive on would have pulled every one down

    Scenario: IX8. Google Drive
      Then through the Drive connection (raea-api src/connect; Drive is live since 11-11), raea-api lists the Drive's
        pictures and videos (files.list: id, name, mimeType, md5Checksum, imageMediaMetadata, videoMediaMetadata,
        thumbnailLink) and hands the Mac each thumbnail through the API; the Mac embeds it. The token stays in raea-api
      And a Drive of 20,000 pictures is searchable without one original downloaded; an original streams through the API
        (LB6's job media route) only when opened or deep-indexed
      And Google Drive for desktop's own folder (~/Library/CloudStorage/GoogleDrive-*), when it is there, is a folder
        source like any other

    Scenario: IX9. The phone itself, later
      Then 0.1.5 reaches the phone's library through iCloud Photos on the Mac. Indexing on the iPhone in raea's iOS app
        (PhotoKit on the device, its Neural Engine, the same model) waits for its Photos usage string and a TestFlight
        build in App Store Connect

  Rule: Plans

    Scenario: IX10. Free keeps the index on the Mac; Pro and Team also keep it in raea's cloud
      Then on Free every source is indexed and searched on the Mac, and nothing leaves it (Libraries' local index)
      And on Pro and Team the index (vectors, thumbnails, the words and labels, never the media) also goes to raea's
        cloud through raea-api, so the library is searchable from raea.io, the iPad and the account's other Macs; a
        Team's workspace index is shared by its members
      And the cloud index counts toward the plan: its thumbnails and vectors as storage bytes (about 20 KB an item),
        shown on Settings → Storage beside media; what each plan includes is Shane's call (open question 1)

  Rule: Managing the index

    Scenario: IX11. Settings → Index says what's indexed, where, by which model, and lets you change it
      Then per source: found, fast-indexed, deep-indexed, waiting and failed, with its speed while it runs; pause,
        resume, index again, and remove (its rows and thumbnails go; the files are never touched)
      And per model (siglip2-b16-256@768, embeddinggemma-2@768): rows, bytes on disk, bytes in the cloud, and its
        version; a model's vectors can be dropped and made again; a newer model makes its rows beside the old ones
        and search moves over once it is done
      And where it all lives (~/.raea/library), and a button that shows it in Finder

    Scenario: IX12. The wow: the counter
      Then turning a source on starts the fast pass at once and shows it: items found, items searchable, pictures a
        second, and the newest thumbnails landing as a wall; search works from the first batch
      And the first run on a library of 10,000 photos is searchable in about a minute on an M-series Mac

  Rule: How it ships

    Scenario: IX13. Where the work runs
      Then the engine does the work, in its own crate (systems/index, raea-sys-index: sources, decode, embed, store,
        search) on raea-sys-vision's Core ML (CpuAndNe), and `raea index` drives it: add, status, search, serve (a
        resident process the feed's host keeps, JSON lines)
      And the feed shows it: the Index plugin (its sources' switches and the counter), Settings → Index, and search,
        which asks `raea index search` and the deep index and fuses them
      And agents reach it as they reach Libraries: `raea library search` and the feed's library tools read both passes

    Scenario: IX14. Measured, not guessed
      Then bench/index runs the fast pass on a fixed public set (pictures and clips under permissive licences) and
        prints pictures a second, video minutes a second, peak memory and recall@5 for a fixed list of queries; a
        release doesn't regress it

  Rule: Open questions for Shane
    Scenario: To decide
      Then 1. What Pro and Team include for the cloud index (items or bytes), and what past it costs
      And 2. Whether Photos' "include photos only in iCloud" is on by default (it fetches each one's small copy over
        the network)
