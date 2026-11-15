@spec
Feature: React Three Fiber both ways: an R3F scene drawn by raea, a raea composition drawn in R3F
  Goal, 2026-11-06: "complete r3f support so that raea can also render to r3f or use r3f as input in raea, two way
  complete r3f support, and measure so that it is complete and nothing is missing". The code is sdk/r3f (raea/r3f for
  authors). Both ways go through raea's scene contract, the `data-3d` elements a frame renders: an R3F scene is read
  into those elements (input), and those elements are drawn as a three.js scene (output). The same units, shapes and
  tone mapping hold on both sides, and both are measured: every kind and class has a row (R3F7) and every case is
  held to the other renderer by SSIM (R3F8).

  Rule: R3F as raea's input

    @done
    Scenario: R3F1. An R3F scene in a raea composition
      Given an R3F scene, the JSX a <Canvas> holds (meshes, lights, cameras, useFrame, refs, primitives)
      When a composition puts it inside <R3F camera={...} shadows> from raea/r3f
      Then raea's engine draws it: each frame R3F's reconciler builds the three.js scene off screen (a stand-in
        renderer, no WebGL), raea steps R3F's clock to the frame and reads the scene into raea's parts
      And useFrame runs once a frame, 1/fps a step, from frame 0; a seek back starts again from 0, so a render, a
        seek and a play give the same frame (test/input.test.tsx "the clock")
      And raea's own hooks work inside it (useCurrentFrame, a context from outside the canvas): the context bridge
        re-provides the composition's contexts to R3F's tree

    @done
    Scenario: R3F2. What is read from three.js
      Then input/kinds.ts has a row for every Object3D and Material class three exports (62 in three 0.180):
        42 drawn, 12 near (a note says what differs), 8 with nothing to draw (a bone, a camera that isn't drawn with,
        a sound, a base class)
      And meshes keep their triangles, normals, uvs (v flipped: Bevy counts from the top), vertex colours, geometry
        groups (a mesh a material); skinned and morphed meshes are posed on the CPU each frame; an InstancedMesh is
        Instances while its copies hold still; Points, Lines, Sprites and three's helpers are drawn
      And lights convert by units.ts, so a three light and the raea light it becomes put the same light on screen; a
        HemisphereLight is an ambient light of its colours' average plus half their difference from straight up
      And textures keep their url; a DataTexture or a canvas becomes a PNG; repeat, offset, rotation and samplers hold

  Rule: raea drawn in R3F and three.js

    @done
    Scenario: R3F3. A raea composition inside an R3F <Canvas>
      Given a raea composition's component
      When an R3F app puts <Raea component={Edit} /> inside its <Canvas>
      Then its 3D is drawn by three.js in that canvas: the component renders as under raea (into a page off screen,
        at its frame), and Scene3d draws the page's parts into a three.js scene R3F holds
      And its camera becomes R3F's, its tone mapping, exposure, shadows, fog, environment and background the canvas's
        (camera={false} and settings={false} leave them to the app)
      And its frame follows R3F's clock at its fps, or `frame` pins it

    @done
    Scenario: R3F4. Scene3d, the three.js host, for any three.js app
      Then sdk/r3f/src/three draws a raea page into a three.js scene without React Three Fiber (sdk/playable's host
        is built on it): Scene3d.sync(page, seconds), fit(width, height), apply(renderer)
      And three/kinds.ts has a row for every kind raea's SDK renders (58): 27 drawn, 11 near, 4 settings, 9 effects
        and 7 that are Bevy's own (a WESL shader or frame pass, a compute pass, a Bevy component or resource,
        volumetric fog and fog volumes)
      And an ECS world (<Entities>) is drawn: its meshes and their transforms come from raea/ecs's own store in
        JavaScript (sdk/app's scene(), the engine's composition of a child under its parent), each mesh's shape by
        three's geometry or core/shape's wasm
      And the 7 that are Bevy's own aren't three.js's to draw: WESL and WGSL run on Bevy's renderer, components and
        resources are Bevy's data, and the fogs are raymarched through Bevy's shadow maps and depth. A page that needs
        them uses raea-web, raea's engine itself in the browser on WebGPU (R3F10); Scene3d names each one it leaves out
        (a warning, and its row)
      And a Model with `at` is a copy of the model at each placement, as raea's engine draws it
      And raea's default tone mapping, TonyMcMapface, is drawn with Bevy's own LUT (sdk/playable's tony.bin) in a
        pass in place of three's output pass; ACES, AgX, Reinhard and none are three's own

    @done
    Scenario: R3F5. raea's solids on the web
      Given a Mesh whose shape is a solid (minus, hull, extrude, Text3D: core/shape's expression)
      Then engine/shape-web compiles core/shape to wasm (tools/shape-wasm.sh, 2.6 MB) and loadShapes() brings it in:
        the solid's own triangles, the same as raea's engine meshes
      And until it loads, a solid waits (it isn't drawn as something else)
      And the public repository's releases carry the module, so an app needn't build Rust: tools/release-web.sh builds
        raea_shape_web.js and raea_shape_web_bg.wasm (and raea-web's raea_web.js and raea_web_bg.wasm, with SHA256SUMS)
        from a commit and attaches them to its GitHub release: every engine release (tag vVERSION; 0.1.0-canary.1 and
        canary.2 were given their own shape code's module on 11-06, without raea-web, which came after them), and a
        pre-release web-SHA for a public commit between them (SHA the public commit's short hash), at
        https://github.com/shinyobjectz/raea/releases/download/TAG/raea_shape_web.js. GitHub serves them as
        downloads (no CORS), so an app serves the two files itself and calls loadShapes("/raea_shape_web.js"), or
        imports the glue and passes the module: loadShapes(await import("./raea_shape_web.js"))

    @done
    Scenario: R3F6. Frame effects
      Then three/effects.ts draws raea's effects with three's EffectComposer in Bevy's order: ambient occlusion,
        outlines, depth of field, bloom (energy-conserving, as Bevy's) and motion blur in the scene's light; Bevy's
        grade (fringes, vignette, white balance, levels) before tone mapping; saturation, looks and lenses after
      And <Raea> runs it whenever the composition has effects; effects={false} hands them to the app's own composer

  Rule: Measured

    @done
    Scenario: R3F7. Nothing is left without a row
      Then test/coverage.test.tsx reads the SDK's sources for every kind it renders and fails on one missing from
        three/kinds.ts, draws every kind three.js draws and fails on a warning, and fails on an effect row the
        composer doesn't draw
      And it walks three's exports and fails on an Object3D or Material class missing from input/kinds.ts
      And it fails on a kind three/kinds.ts calls "engine" that has no raea-web parity case (R3F10)
      And `bun test` in sdk/r3f runs these with the input, round-trip and source-export tests (27 tests)
      And a ShadowMaterial is raea's <Mesh shadow="catch">: a surface that shows only the shadows falling on it, in
        its material's colour and opacity (systems/material shadow_catch.rs in the engine, three's ShadowMaterial in
        Scene3d)

    @done
    Scenario: R3F8. Each way held to the other renderer
      Given test/parity/fixtures.tsx: R3F scenes (in-*), raea compositions (out-*) and the same compositions written
        as R3F source (src-*, R3F9), at 640x360
      When tools/parity.sh renders each with `raea render` (native.tsx) and with three.js in headless Chrome
        (test/parity/page.tsx, which tools/serve.ts builds and serves itself, with the wasm tools/shape-wasm.sh builds),
        and compares the two by SSIM
      Then every case is at least 0.92 (measured 11-07)
        | case        | what                                                                 | SSIM   |
        | in-basic    | a spinning box (useFrame), PBR and clearcoat, shadows, a point light | 0.9917 |
        | in-lights   | spot, hemisphere and area lights, fog, Lambert and flat Phong        | 0.9795 |
        | in-things   | InstancedMesh, multi-material, edge lines, a sprite                  | 0.9917 |
        | in-catch    | a ShadowMaterial floor: only the box's shadow over the background    | 0.9966 |
        | in-lettra   | lettra's MSDF text (TSL) and a TSL gradient on its clock             | 0.9994 |
        | in-lettrafx | lettra's wipe and scramble (an attribute, a uniform array)           | 0.9992 |
        | in-shader   | ShaderMaterials: a rippled sphere, points sized by an attribute      | 0.9989 |
        | in-post     | <EffectComposer>: HueSaturation, Vignette, ToneMapping ACES          | 0.9451 |
        | in-drei     | drei's Text, Environment preset, ContactShadows, Float, useGLTF      | 0.9932 |
        | in-approx   | matcap, normal, Phong specular, dashes, clipping, hemisphere, probe  | 0.9848 |
        | out-basic   | a raea scene, ACES, a moving box                                     | 0.9958 |
        | out-tony    | the same scene in raea's default tone mapping, TonyMcMapface         | 0.9961 |
        | out-lights  | spot and area lights, fog, Text3D (a solid)                          | 0.9633 |
        | out-bloom   | <Bloom intensity={2}>                                                | 0.9295 |
        | out-grade   | <Grade> saturation, contrast, vignette                               | 0.9738 |
        | out-lens    | <LensDistortion k1={0.15}>                                           | 0.9922 |
        | out-catch   | <Mesh shadow="catch">, three's ShadowMaterial                        | 0.9972 |
        | out-entities| <Entities>: a raea/ecs world, a ship and its gun turned by a system  | 0.9972 |
        | src-basic   | out-basic exported as R3F source, drawn by a plain <Canvas>          | 0.9958 |
        | src-tony    | out-tony exported (Tony through raea-effects.ts)                     | 0.9961 |
        | src-lights  | out-lights exported as R3F source (Text3D as its triangles)          | 0.9633 |
        | src-bloom   | out-bloom exported (raea-effects.ts)                                 | 0.9295 |
        | src-grade   | out-grade exported                                                   | 0.9738 |
        | src-lens    | out-lens exported                                                    | 0.9922 |
        | src-catch   | out-catch exported                                                   | 0.9972 |
        | src-entities| out-entities exported                                                | 0.9972 |
      And three r181 rewrote UnrealBloomPass (mrdoob/three.js#31528): three/effects.ts puts r180's kernels, composite
        and blend back on three's pass, so out-bloom holds to the bloom raea's matches (three's frame equals r180's at
        SSIM 0.999998)
      And three's ACES scales its input by 1/0.6 where Bevy's doesn't: the host's exposure takes 0.6 under ACES, and
        an R3F canvas's ACES reads as 0.74 stops more (measured: out-lights 0.927 to 0.963, in-lights 0.927 to 0.979)

  Rule: raea written as R3F

    @done
    Scenario: R3F9. A raea composition exported as React Three Fiber source
      Given a raea composition
      When `bun sdk/r3f/tools/export.ts EDIT.tsx COMPOSITION OUT.tsx [--from N] [--frames N | --all]` runs, or
        toR3FSource(component, options) from @raea/r3f/export
      Then OUT.tsx is a TSX module that needs only react, three and @react-three/fiber: the scene as JSX (meshes with
        three's geometry and materials, lights, fog, background, environment, models by url), what moves as a table a
        frame (position, rotation, scale, visibility, materials, lights, the camera) applied in useFrame, and a default
        export that is a <Canvas> with raea's camera, tone mapping, exposure and shadows
      And a solid (minus, hull, extrude, Text3D) is written as its triangles, meshed by core/shape's wasm
      And when the scene has frame effects or raea's Tony tone mapping, raea-effects.ts is written beside it: the
        composer <Raea> draws them with (three/effects.ts and Bevy's Tony table, needing only three), run by the module
        after each frame's updates; effects that only raea's engine draws (three/kinds.ts "engine") are named in the
        header
      And test/export.test.tsx checks what is written; the src-* cases (R3F8) draw it against raea's render

  Rule: What three.js can't draw, raea-web does

    @done
    Scenario: R3F10. The engine's own kinds held to `raea render` in the browser
      Given sdk/web/test/parity/cases.ts: a case for each kind three/kinds.ts calls "engine", and <Entities>
      When sdk/web/tools/parity.sh renders each with `raea render` and with raea-web (sdk/web, raea's engine on WebGPU)
        in headless Chrome (tools/shoot.ts), and compares the two by SSIM
      Then every case is at least 0.92, and a blank web frame fails whatever its SSIM (measured 11-06)
        | case           | kinds                     | SSIM   |
        | surface-shader | SurfaceShader             | 0.9998 |
        | frame-pass     | FramePass                 | 1.0000 |
        | compute        | Compute                   | 1.0000 |
        | component      | BevyComponent             | 1.0000 |
        | resource       | BevyResource              | 0.9999 |
        | entities       | Entities                  | 0.9760 |
        | fog            | VolumetricFog, FogVolume  | 0.9993 |
      And the first five are sdk/raea/bench/bevy-ts's cases (shader-scene, frame-pass, compute, reflect-on-camera,
        reflect-keyed-resource-still, 64x64); entities and fog are 320x180
      And a page's files (a glTF, a picture) are fetched from assets/ beside the page, Bevy's asset folder on the web
      And measuring it fixed raea-web: a model's file is fetched rather than looked for on disk (runtime/scene3d
        model.rs), a black page background is the 3D's clear rather than a cover over it (rgb(0, 0, 0) was read as
        see-through, here and in Scene3d), and a layered <Canvas3D>'s empty slot isn't drawn

  Rule: What R3F apps are made of draws in raea

    Goal, 2026-11-06: "finish the r3f": custom GLSL shaders, @react-three/postprocessing, the materials and lights that
    were approximate, and a drei-heavy scene, each read by <R3F> and held to three.js by parity (R3F8).

    @done
    Scenario: R3F11. three.js shaders run in raea's engine
      Given a ShaderMaterial or RawShaderMaterial in an <R3F> scene (written by hand, or drei's shaderMaterial())
      When raea reads it (src/input/shader.ts)
      Then its GLSL goes as three would compile it: the #includes expanded from three's ShaderChunk, its defines in
        front, the #ifdef blocks its defines settle decided, its uniforms as this frame's numbers and its textures as
        pictures (a picture made in code as a PNG data: URL)
      And raea-sys-three-shader translates it with naga and runs it with three's built-ins (the matrices, the camera,
        a colour and uvs on every vertex, up to 4 attributes of its own, 64 vec4 uniforms, 4 textures)
      And a shader that includes three's tone mapping is tone-mapped by Bevy's, one that doesn't is shown as three's
        canvas shows it
      And in-shader (a sphere rippled by a uniform set in useFrame, points sized by their own attribute): SSIM 0.9989

    @done
    Scenario: R3F12. @react-three/postprocessing's <EffectComposer> as raea's frame effects
      Given an <EffectComposer> in an <R3F> scene
      When raea reads its effects (src/input/post.ts)
      Then each becomes the raea effect that does the same thing: Bloom; Vignette, ChromaticAberration,
        HueSaturation and BrightnessContrast in one Grade; ToneMapping; DepthOfField and Bokeh; SMAA and FXAA; SSAO
        and N8AO; Pixelation; LensDistortion
      And one raea has nothing for (noise, glitch, god rays ...) is named once and left out (kinds.ts's EFFECTS
        decides every effect postprocessing exports, tested)
      And a composer's frame is tone-mapped only by its ToneMapping effect, as in three
      And in-post (HueSaturation, Vignette, ToneMapping ACES): SSIM 0.945

    @done
    Scenario: R3F13. A drei-heavy scene: <Text>, <Environment preset>, <ContactShadows>, <Float>, useGLTF
      Given test/parity's in-drei case: a knot loaded by useGLTF in a <Float>, troika's <Text> in Inter, drei's
        <ContactShadows> under it and <Environment preset="city"> lighting it
      When raea renders it
      Then three's loaders read their files where a native render has no fetch: FileLoader by the `asset.read` native
        call (a file, or an http(s) URL through raea's cache), so a glTF is parsed in JS; a DataTextureLoader (HDR,
        EXR) hands back the cached file, which raea's engine loads as <Environment src>
      And what an app draws into a render target is done on the CPU where it's a pass raea knows (src/input/targets.ts):
        a MeshDepthMaterial override from an orthographic camera and three's 9-tap blurs, which are drei's
        <ContactShadows>; anything else drawn into a target is named and left empty
      And a mesh whose transform mirrors it is drawn with its triangles rewound (three flips the front face; Bevy
        culls as wound)
      And raea's bundler gives a composition troika-three-text as sdk/r3f/src/input/troika.ts (engine/bundle's
        STAND_INS): its Text holds troika's settings and draws nothing, and src/input/text.ts reads any troika Text as
        raea's <RichText> (size, the face's family from its name table, weight, colour, anchors, maxWidth)
      And a root whose parts mounted late (they suspended) starts again from frame 0 once its loads are cached, and
        Math.random is seeded per root, so drei's <Float> and three's uuids are the same however often a seek restarts
        it (render equals play)
      And in-drei: SSIM 0.993 (Float at speed 0 in the case: three's page draws its phase at random)

    @done
    Scenario: R3F14. The approximate kinds made exact or closer
      Given test/parity's in-approx case
      When raea renders it
      Then MeshMatcapMaterial and MeshNormalMaterial run three's own ShaderLib programs (R3F11), so they look as three
        draws them
      And Phong's specular colour is its F0 (three's Blinn-Phong): Bevy's reflectance and specular tint
      And a LineDashedMaterial is cut into its dashes by the line's distances (computeLineDistances), as three's shader
        cuts it
      And clipping planes (a material's with localClippingEnabled, the renderer's, a ClippingGroup's) cut the mesh's
        triangles, so what three discards isn't drawn (src/input/clip.ts; clipIntersection keeps what is inside any one)
      And drei's MeshTransmissionMaterial draws with Bevy's transmission (its own uniform's, as three's is 0 under it)
      And a LightProbe is an ambient light of its constant band and a light along its linear band (exact for surfaces
        facing that light or sideways, as the hemisphere light is), and its constant band is scaled as three's
        irradiance is (0.886, where 0.282 made it a third as bright)
      And in-approx: SSIM 0.9848 (Phong's F0 is held at Bevy's highest, 0.16, so its orange highlight glints less)

    @done
    Scenario: R3F15. What an R3F scene throws is the composition's error (#131)
      Given an <R3F> whose scene throws: rendering a component, in an effect, in a ref
      Then the frame throws it out of the <R3F> drawing it (src/input/session.ts: the scene under raea's own error
        boundary), so the render is refused naming it, exactly as a throw outside R3F is: "REFUSED the composition threw
        at frame 0: Error: ...", exit 1
      And never as R3F's own root reported it (reportError, else a stack without its message), the scene drawing nothing
        and the render ending with 0 errors
      And three's PMREMGenerator makes no GPU cube map in raea (src/input/pmrem.ts): fromEquirectangular of a panorama
        raea loads is that panorama, lighting the scene as scene.environment does; fromScene (RoomEnvironment) and
        fromCubemap are left out with a warning naming them, the scene drawn without them
      And test/errors.test.tsx holds it

  Rule: What three.js work online is made of draws in raea (0.1.5)

    Goal, 2026-11-15: "ship all of this in 0.1.5". An audit that day rendered 25 probes of what people post online made
    with three.js and R3F (TSL compute particles, render targets, plain three.js with its own renderer, drei's showpiece
    helpers, postprocessing) through 0.1.4's <R3F>: six refused the render, five drew wrong, moving instances cost a
    mesh a copy, and the raea skill told agents R3F doesn't exist here. Each scenario below is one of those, held by a
    case in test/parity (SSIM against three.js in Chrome, R3F8) or a test that refuses the old behaviour.

    @done
    Scenario: R3F16. What three can't draw in raea is named and drawn, never refused
      Given an <R3F> scene with any of: a TSL node material on an InstancedMesh, a compute dispatched through `gl.compute`,
        drei's <MeshReflectorMaterial>, drei's <Environment> with children (Lightformers), drei's <Stars>, drei's <Clouds>
      When raea renders it
      Then the render isn't refused for what raea's stand-in renderer lacks: it answers what three's renderers ask as they
        draw (a WebGPU backend's device limits, WebGLRenderer's `state`, `compute`, a CubeCamera's coordinate system)
      And what raea can't draw yet is named once in a warning and drawn as the documented fallback (flat, or its base
        material), the rest of the scene drawn as three would
      And a ShaderMaterial on Points (drei's <Stars>) draws its points with its shader
      And a texture three loads from an http(s) URL is fetched through raea's cache, never read as a path in the project
      And test/errors.test.tsx renders each of the six without a refusal
      Status (11-15): session.ts's stand-in WebGLRenderer answers coordinateSystem (WebGL's), `state` (an inert GL state:
        every call does nothing), `extensions`, a full `info`, and takes `compute`/`computeAsync` into gl.__raea.computes
        (warned once: R3F22 runs them); tsl.ts's stand-in WebGPURenderer has a backend device with wgpu's limits, any Error
        building a graph is named and the material drawn flat, and a graph reading an attribute the drawn mesh can't carry
        (an InstancedMesh's matrices past 1,024 copies, an InstancedBufferAttribute) is refused there, not by the engine;
        shader.ts gives a sampler2D the material never sets a 1x1 opaque black picture (WebGL's unbound read); texture.ts
        fetches an http(s) picture through raea's cache (`asset.load`). The six probes from the audit render (stars, clouds with the CDN's cloud.png, the
        reflector's plane as its base material, Environment+Lightformer without the cube map, TSL on 100 and 2,000
        copies, gl.compute), each named; test/errors.test.tsx 10/10

    @done
    Scenario: R3F17. A sky at three's far plane is drawn behind the scene
      Given a shader that puts its vertices on the far plane as three does (`gl_Position.z = gl_Position.w`, drei's and
        three's <Sky>, any skybox ShaderMaterial), or a node material's WGSL doing the same
      When raea translates it (Bevy's projection is reverse-Z: the far plane is z = 0)
      Then it is written onto reverse-Z's far plane, so the scene in front of it shows
      And in-sky (three's Sky behind a box): SSIM at least 0.92 against three.js
      Status (11-15): shader.ts farPlane() rewrites `X.z = X.w;` and `gl_Position = X.xyww;` to z = 0 in a ShaderMaterial's
        vertex GLSL and in a node material's vertex WGSL (material.ts, on the part tsl.ts makes); in-sky SSIM 0.9986 (in-shader 0.9935, in-drei 0.9932,
        in-approx 0.9848 unchanged); test/errors.test.tsx holds the rewrite

    Scenario: R3F18. Agents are taught <R3F>
      Given the raea skill (raea-skills skills/raea) and its source guides (sdk/guides/craft/bevy)
      When an agent reads what 3D in raea can be written with
      Then 3d/r3f.md says when to use <R3F> (an R3F or three.js scene, drei, postprocessing, GLSL or TSL shaders), how to
        set up a project for it, what draws exactly, what differs, and what raea's own parts do better
      And the lines saying useFrame, refs and drei "don't exist" say instead that they don't exist in raea's own 3D parts
        and do inside <R3F>
      And SKILL.md's router sends an agent with three.js or R3F code to 3d/r3f.md

    Scenario: R3F19. Plain three.js with its own renderer
      Given a composition whose three.js code makes its own renderer, as single-file demos do: `new
        THREE.WebGLRenderer({ canvas })` or three/webgpu's `WebGPURenderer`, a scene and camera, `renderer.render(scene,
        camera)` from `setAnimationLoop` or requestAnimationFrame, `document.body.appendChild(renderer.domElement)`
      When raea renders or plays it
      Then raea's bundler gives it a renderer that draws nothing and records what it was asked to render, and each frame
        that scene is drawn by raea's engine as <R3F>'s scene is, in the canvas's box
      And its animation loop steps 1/fps a frame from the composition's frame (render equals play)
      And in-vanilla (a plain three.js page): SSIM at least 0.92 against the same page in Chrome

    Scenario: R3F20. An InstancedMesh's moving copies are one instance buffer a frame
      Given an InstancedMesh whose matrices or colours change every frame (a field of 20,000 copies moved in useFrame)
      When raea draws it
      Then its copies are one row drawn by Bevy's instancing, their transforms and colours that frame's buffer, never a
        mesh a copy
      And 20,000 moving copies at 640x360 take at most 100 ms a frame (0.1.4: 1,220 ms), 2,000 at most 40 ms (0.1.4: 94)
      And in-instanced (2,000 moving copies with colours): SSIM at least 0.92 against three.js
      # The plan (r3f-gpu, 11-15). 0.1.4's cost is each copy a Mesh with its own BufferMesh: 20,000 copies of a box's
      # arrays as JSON a frame. raea's <Instances> already moves a set frame by frame (its data-at list a frame, the
      # row's inst_all/inst_k, scene3d's sync_instances: one entity a copy sharing the mesh and material, which Bevy's
      # automatic batching draws as one instanced call), so the frame's buffer is one column holding that frame's list,
      # not keyed columns per copy. What it lacks is a colour a copy, and a list that isn't text parsed every frame.
      #   R3F20a. raea/3d <Instances>: an Instance takes `color` (a hex string, or linear [r, g, b]), multiplied with the
      #     material's as three's instanceColor is; a set with colours packs 12 numbers a copy, written as data-at
      #     {"stride":12,"at":[...]}; runtime/look's inst column is then 14 numbers a copy (placement, linear rgba) with
      #     inst_tint 1. scene3d: a tinted set's copies share one TintMaterial (raea-sys-material tint.rs: the standard
      #     material extended by a storage buffer of the copies' colours, read by the copy's MeshTag above the two bits
      #     compose's pass keeps), the buffer rewritten when the frame's list changes; the parent's standard material
      #     copied into its base when it changes (sync_meshes recolours the set as before). compose's sync_passes keeps
      #     a tag's high bits.
      #   R3F20b. sdk/r3f scene.ts: an InstancedMesh with a standard surface is always one <Instances>, still or moving,
      #     coloured or not: its copies from the instance matrices (instanceMatrix.count of them, those past `count` at
      #     scale 1e-6 so the count holds), instanceColor as each copy's colour. A ShaderMaterial or several materials
      #     keep the mesh-a-copy path.
      #   R3F20c. Speed: each render list parsed once into transforms (scene3d Placements), not each frame it shows;
      #     look packs without reformatting text where it can. Measured before and after on 2k and 20k (probes
      #     inst-2k-48f, inst-20k-48f), with RAEA_FRAME_LOG's react/sync/draw split.
      #   R3F20d. in-instanced in test/parity (2,000 moving copies with colours under a directional light), SSIM >= 0.92.

    @done
    Scenario: R3F21. A lit TSL node material is lit by the scene's lights
      Given the plan
        | crate                 | file                                | lines | job                                                    |
        | raea-sys-three-shader | systems/three-shader/src/surface.rs | 90    | the lit fragment entry: three's surface as Bevy's PbrInput, lit |
        | sdk/r3f               | sdk/r3f/src/input/tsl-lit.ts        | 85    | three's builder writing the surface, the lit contract  |
      And a MeshStandardNodeMaterial or MeshPhysicalNodeMaterial (or a Lambert, Phong or toon one) with nodes of its
        own (colorNode, positionNode, normalNode, roughnessNode, metalnessNode, emissiveNode, opacityNode)
      When raea draws it
      Then three's builder writes its surface with no lights of three's: its output (three's `context.getOutput`) is
        the colour and opacity, the view-space normal and roughness three lights with, the emission and metalness, and
        the point in world space after positionNode, returned by the fragment stage as `three_surface` (tsl-lit.ts)
      And three-shader's lit entry makes Bevy's PbrInput of it and runs the standard material's lighting, fog and tone
        mapping (surface.rs): lit by Bevy's lights and shadows as raea's StandardMaterial is, in three-shader's own
        material rather than SH1's ShaderExt, so the vertex stage is three's (positionNode moves it) and its uniforms
        have 254 slots, not 16
      And a Lambert surface is fully rough and Phong's shininess a roughness, neither metal; a basic, shadow or volume
        node material, or one with a fragmentNode, keeps three's own output
      And in-tsl-lit (a noise-displaced, noise-coloured icosahedron under a directional light): SSIM at least 0.92
      And the status, 2026-11-15: built as planned. in-tsl-lit SSIM 0.9906 against three.js (WebGPURenderer, its WebGL 2
        backend) in Chrome at frame 12, where 0.1.4 drew the same blob black (the 11-15 audit's tsl-displace). Proof:
        sdk/r3f test/tsl.test.ts (the lit contract: three_surface returns base, normal, emissive and world, no three
        light code in the graph, the instance carried flat; a Lambert surface rough and unmetallic) and
        raea-sys-three-shader's surface test; tools/parity.sh runs in-tsl-lit. A displaced mesh's shadow is still its
        undisplaced shape (three-shader's shadow pass is the standard one), and a fragmentNode keeps three's output

    Scenario: R3F22. TSL compute and storage buffers
      Given TSL storage (`instancedArray`, `storage`, `attributeArray`), compute nodes (`Fn(...)().compute(n)`) run with
        `gl.compute` / `computeAsync` in useFrame, and a material reading the buffers (`.element(instanceIndex)`,
        `.toAttribute()`) on an InstancedMesh, Points or Sprite
      When raea renders it
      Then each compute's WGSL (three's WGSLNodeBuilder) runs as a raea <Compute> once each frame it was dispatched, in
        the order dispatched, its storage buffers kept from frame to frame and shared with the materials that read them
      And a render that starts past frame 0 steps every compute from frame 0, so a render equals its play
      And in-tsl-particles (65,536 particles orbited by a compute): SSIM at least 0.92 against three.js on WebGPU

    Scenario: R3F23. Render targets are drawn on the GPU
      Given what an app draws into a render target itself (`gl.setRenderTarget(t); gl.render(scene, camera)`, drei's
        useFBO, <RenderTexture>, <MeshReflectorMaterial>, <MeshPortalMaterial>, three's GPUComputationRenderer, a
        ping-pong between two targets, a feedback or trail effect)
      When raea draws it
      Then that render is a camera of raea's engine drawing that scene into an image the frame's materials read the
        target's texture as, before the cameras that read it, in the order the app drew them
      And a fullscreen quad drawn into a target (a GPGPU pass, a blur) is that quad's shader run into the image
      And in-fbo, in-pingpong (a GPUComputationRenderer wave) and in-reflector: SSIM at least 0.92 against three.js

    Scenario: R3F24. onBeforeCompile patches
      Given a material patched by onBeforeCompile (drei's MeshWobbleMaterial and MeshDistortMaterial, a patched
        MeshStandardMaterial's vertex displacement)
      When raea draws it
      Then the patched GLSL is what runs: three's program for that material with the patch applied, translated as R3F11's
      And in-patched: SSIM at least 0.92 against three.js

    Scenario: R3F25. Post-processing: three's TSL passes and postprocessing's other effects
      Given three/webgpu's PostProcessing (`pass(scene, camera)` with bloom, GTAO, SSR, TRAA, DOF, film, ... nodes) or
        any effect @react-three/postprocessing exports
      When raea renders it
      Then each becomes raea's effect that does the same (Bloom, AmbientOcclusion, DepthOfField, AntiAliasing, the
        ray-traced reflections) or a <FramePass> raea ships for it: Noise, Scanline, Sepia, DotScreen, ColorAverage,
        ColorDepth, Grid, Glitch, ShockWave, TiltShift, ASCII, LUT, GodRays, Outline
      And none is left out without a warning; in-post2 and in-tslpost: SSIM at least 0.92 against three.js

    Scenario: R3F26. <R3F> in a box smaller than the page (#130)
      Given an <R3F> in a box of its own inside a larger page
      When raea draws it
      Then its camera's aspect and projection are the box's, centred on the box, as R3F's <Canvas> is in a browser
      And #130's five cases each draw the plane at its expected size and place

    Scenario: R3F27. R3F's pointer events and controls in a live play
      Given an <R3F> in a game played live: meshes with onClick, onPointerOver, onPointerMove; drei's <OrbitControls>
      When the player moves and clicks the pointer over the 3D
      Then R3F's events fire on the mesh the pointer is over (raycast from raea's pointer through the <R3F>'s camera), and
        controls move the camera, while a render (no pointer) is unchanged
      And a recorded input log replays to the same frames
