@spec
Feature: A render in layers: captions, text, graphics, footage and sound apart, for CapCut, Premiere, Resolve and raea's timeline
  wackyza, 2026-11-16: "export a rendered video, in all its component layers (video, audio, graphics, all separate) to
  Premiere, Resolve and Capcut ... keep graphics on one overlay track. Titles aka text elements should be their own
  track. And captions, if applicable, should be their own track. I think this separation would be valuable for the
  internal (in-app) timeline as well ... the best order of events might be to get the layers rendering really robust,
  then be able to push it to the in-app timeline and all the ext editors".

  A cut finished in another editor needs its parts, not one flattened movie. `raea export capcut` (docs/capcut.feature)
  already hands CapCut the footage and sounds as clips of their own files and everything raea draws as one
  transparent overlay. This spec takes that split out of the CapCut export, makes it finer (captions and text apart
  from the other graphics), proves it against the render, and gives it to every editor from one place: CapCut,
  Premiere Pro and DaVinci Resolve, and the layout of raea's own timeline.

  What there is to build on (read 11-16):
    - apps/capcut: the manifest's frames grouped into runs of footage and sound (plan.rs), the overlay rendered with
      the footage hidden (RAEA_HIDE_MEDIA=1) and what's under it (RAEA_HIDE_MEDIA=under), written as a CapCut draft
      (io/capcut). Its plan is written in CapCut's types, so no other editor can use it as it is.
    - systems/layers: hide.rs hides layers by key (RAEA_HIDE_LAYERS) and the footage (RAEA_HIDE_MEDIA) as page CSS;
      page.rs already calls an element whose children are all text "text", and footage "video".
    - io/alpha: transparent movies without ffmpeg (CC1): ProRes 4444, HEVC with alpha, QuickTime Animation, PNG.
    - sdk/timeline: tracks by kind (effect, text, overlay, main, sound, TL1a); the captions operation (TL7c) puts
      lines on a track named Captions, and read.ts writes each line into the composition as a `text` op, the same op
      a title is written with, so once in the code a caption and a title can't be told apart.

  The layers, top to bottom, and what each is:
    | layer    | what it holds                                                  | what the editor gets                 |
    | captions | the words said, as raea draws them (CA1)                       | a transparent movie + an .srt beside |
    | text     | titles and other words on the picture that aren't captions    | a transparent movie                  |
    | graphics | everything else raea draws over the footage: shapes, 3D, FX    | a transparent movie                  |
    | footage  | each <Video>'s file, at its trims, place and size (CC2a)       | clips of the original files          |
    | under    | what raea draws beneath the footage: the backgrounds of what   | a movie, left out when all black     |
    |          | holds it (CC2b)                                                |                                      |
    | sound    | each sound clip, and footage's own sound linked to its clip    | audio clips of the original files    |
  A layer with nothing in it is left out, and costs no render.

  The passes cost time only at export. Rendering a composition, in the app or with `raea render`, draws it once, as
  it does today; the marks LA1 adds are attributes in the code and draw nothing; raea's timeline reads the code (TL0c)
  and renders no layer. Only `raea export` (layers, capcut, premiere, resolve) renders the layers, each pass once.

  Background:
    Given a composition renders with --manifest (render.json), as docs/capcut.feature's Background says
    And the reference render is the composition as `raea render` draws it, every layer at once

  Rule: LA1. Captions and text are marked, so every pass and every tool tells them apart
    Scenario: LA1a. A caption carries data-raea-role="caption"
      Given words drawn as captions: lines the captions operation (TL7c) wrote, or a caption the SDK or an agent draws
      Then the element holding each caption's words carries data-raea-role="caption", and so does everything inside it
        (a word highlighted as it's said is part of its caption)
      And the captions operation's `text` ops say they are captions, and the code they write keeps the mark, so a
        timeline read back from the code (TL0c) puts them on the Captions track again
      And the raea skill's caption guide says to mark captions this way, and `raea check` names words that look like
        captions (timed to the transcript, one line at a time, at the bottom third) and aren't marked

    Scenario: LA1b. Text is what Premiere, Resolve and CapCut call text
      Given a text item in each editor is words and their own style: Premiere's text layer (Essential Graphics),
        Resolve's Text+, CapCut's text, each with its font, fill, stroke, shadow and a background box of its own
      Then an element is text when it carries data-raea-role="text", or when it isn't a caption and its children are
        all words (page.rs's "text"), and it draws as those editors' text does: its words, its stroke and shadow, its
        own background
      And the elements around it (a lower third's bar, a card, a shape) are graphics, as a shape is its own item in
        each editor, so an editor can move the words apart from what they sit on
      And an element can opt out with data-raea-role="graphics" (a logo set in a font, a number in a chart)

  Rule: LA2. Each layer is a pass of its own, rendered transparent, only when exported
    Scenario: LA2o. A render that isn't an export draws once
      Given a composition with captions, text, graphics and footage
      When it is rendered in the app or with `raea render`
      Then it is drawn in one pass, in the same time as before this spec, and no layer is written
      And an export of a version already rendered takes that render as its reference, and doesn't draw it again

    Scenario: LA2a. The passes
      When the composition is exported in layers
      Then it is rendered once for each layer that has something in it, --alpha, with RAEA_LAYER naming the layer:
        | RAEA_LAYER | draws                                                                         |
        | captions   | only captions                                                                 |
        | text       | only text                                                                     |
        | graphics   | what RAEA_HIDE_MEDIA=1 draws today, less captions and text                    |
        | under      | what RAEA_HIDE_MEDIA=under draws today                                        |
      And RAEA_HIDE_MEDIA keeps working as it does, so `raea export capcut` from before this spec is unchanged
      And a pass is cut into clips where it draws something, frames that draw nothing a gap (CC2b)

    Scenario: LA2b. Captions also go out as SubRip
      Then each run of frames a caption shows one line of words is an entry of NAME.srt: its words as the page shows
        them (a word highlight doesn't start a new entry), its first frame and the frame after its last, in timecode
      And the .srt is written beside the export for the person to upload to a network or keep, and no editor's
        project imports it (the captions movie is what the timeline shows, in raea's own style)

  Rule: LA3. The layers put back together are the render
    Scenario: LA3a. The layers composited over each other match the reference render
      Given the layers' movies and the footage's frames as the manifest places them
      When they are composited bottom to top (under, footage, graphics, text, captions), each over the last
      Then every frame matches the reference render within the alpha codec's error (CC1a's numbers), checked at
        least at each clip's first, middle and last frame
      And a frame that doesn't match names the layer and the element (its key, file:line:col) that draws there

    Scenario: LA3b. What can't be pulled apart is moved, and said
      Then a caption or a text element that something drawn after it covers goes down into the graphics layer, so
        what's in front stays in front, and the export says which and why
      And an element whose look depends on what's under it (mix-blend-mode, backdrop-filter, a mask over footage)
        stays in the layer that draws what it mixes with, or is named when no layer can hold it
      And CC2e's notes (filters, rotations, masks, notes and <Mix>) are said as they are

    Scenario: LA3c. A fixture of each kind passes
      Given fixtures: full-frame footage with a title and captions; a lower third (bar and words); a picture in
        picture on a coloured card; captions with a word highlight; a title behind a graphic; 3D over footage
      Then LA3a passes for each, and LA3b's moves and notes are the ones listed for it

  Rule: LA4. One layered timeline, written by `raea export layers`, read by every editor's export
    Scenario: LA4a. `raea export layers EDIT.tsx -o DIR` writes the layers and their timeline
      Then DIR holds layers.json, each layer's movie, NAME.srt when there are captions, and reference.mp4 (the
        reference render, to compare the editor's timeline against; on no timeline)
      And layers.json says the frame rate, size and length, and the tracks top to bottom, each with its layer and
        its clips: the file, where it starts and how long, its first source frame, its place and size, its gain and
        fades, and the id of the clip it's linked to (footage and its own sound)
      And it says what LA3b moved and what CC2e notes, the same lines the command prints
      And it names no editor's format: CapCut's draft, Premiere's and Resolve's XML are written from it

    Scenario: LA4b. The CapCut export reads it
      Then `raea export capcut` is layers.json written as a CapCut draft, with a captions track and a text track
        above the overlay, and apps/capcut's tests pass as before for a composition with no captions or text
      And CapCut's tracks keep CapCut's order (TL1a): text above overlays above the main track, sound below
      And reference.mp4 is copied into the draft's folder, on no track, where CC2d kept it only beside the draft;
        every export's folder holds it, so the person can compare what the editor plays with what raea rendered

  Rule: LA5. Premiere Pro and DaVinci Resolve take the same XML
    Scenario: LA5a. `raea export premiere EDIT.tsx` and `raea export resolve EDIT.tsx` write an FCP7 XML project
      Then each writes a folder: NAME.xml (xmeml version 5), the layers' movies, the .srt (LA2b) and reference.mp4,
        which the sequence doesn't use
      And the sequence has the composition's frame rate and size, its video tracks bottom to top under, footage
        (main, then each picture in picture on a track above), graphics, text, captions, and its audio tracks below
      And each clip of the original footage points at its file where it is, never copied (CapCut's draft copies,
        since sandboxed CapCut reads only its own folder, CC2d), at its trim, scaled and placed to its
        box (Basic Motion), at its gain (Audio Levels)
      And a video clip and its own sound are linked clipitems, so they move together, and each editor's own control
        separates them: in Premiere, Unlink; in Resolve, Link Clips off (Cmd/Ctrl+Shift+L); in CapCut, Extract audio
      And the two commands differ only where Premiere and Resolve read the same XML differently (found in LA5b)

    Scenario: LA5b. Premiere and Resolve open it as raea rendered it
      Given the LA3c fixtures exported
      When the XML is imported (Premiere: File, Import; Resolve: File, Import, Timeline)
      Then every track and clip is where layers.json puts it, nothing is offline, and the frame at each clip's
        middle matches the reference render
      And which alpha codecs stay transparent in each is recorded here, with the version tested, and each command
        uses the first that does

  Rule: LA6. raea's timeline is laid out the same way (raea-feed, after LA1)
    Scenario: LA6a. A timeline read from the code puts captions, text and graphics on their own tracks
      Given a composition read as a timeline (TL0c)
      Then elements marked or found as captions are on the Captions track, text on text tracks and other graphics
        on overlay tracks, as LA1 says, so the timeline and the exports agree on what each element is
      And they stay what TL0c makes them: clips that edit the code, not movies

  Rule: LA7. The skill teaches it
    Scenario: LA7a. The delivery guide teaches layers
      Then sdk/guides/craft/motion/delivery.md says a cut an editor finishes goes out with raea export capcut,
        premiere or resolve, that captions arrive styled on their own track with an .srt beside them, and that words
        are fixed in raea and exported again, since the editors get them as pictures
      And it says to mark captions (LA1a) and to pass on what the export notes
