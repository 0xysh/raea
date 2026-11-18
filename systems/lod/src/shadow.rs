//! The sun's shadows fade out where they end (docs/realism.feature LT21). Bevy blends each shadow cascade into the next
//! over their overlap, but draws everything past the last cascade's far bound fully lit, so a city's shadows stop in a
//! straight line across the street (Shane, 11-17, on the 540p fly: "the shadow having a clear cull distance"). Bevy's
//! shadows.wesl is swapped for one whose last cascade fades to lit over the same overlap (raea_sys_frame's
//! swap_embedded_shader, which checks Bevy's file is still the one this was written against).

use bevy::prelude::*;

/// FNV-1a of bevy_pbr 0.20's render/shadows.wesl, the file the fade was written against
const UPSTREAM: u64 = 0xcd13_e017_3c8e_a62c;

/// the end of fetch_directional_shadow's blend into the next cascade, where the last cascade's fade goes
const FROM: &str = "            shadow = mix(shadow, next_shadow, (-view_z - next_near_bound) / (this_far_bound - next_near_bound));
        }
    }
    return shadow;
}";

const TO: &str = "            shadow = mix(shadow, next_shadow, (-view_z - next_near_bound) / (this_far_bound - next_near_bound));
        }
    } else {
        // raea: the last cascade fades to lit over the cascades' overlap, so the shadows don't end in a line
        let far_bound = (*light).cascades[cascade_index].far_bound;
        let fade_from = (1.0 - (*light).cascades_overlap_proportion) * far_bound;
        if (-view_z >= fade_from) {
            shadow = mix(shadow, 1.0, (-view_z - fade_from) / max(far_bound - fade_from, 0.0001));
        }
    }
    return shadow;
}";

/// Bevy's shadows.wesl with the last cascade's fade
fn faded(bevy: &str) -> String {
    assert_eq!(bevy.matches(FROM).count(), 1, "raea: Bevy's shadows.wesl no longer ends its cascade blend where raea's fade goes (docs/bevy-upgrade.md)");
    bevy.replace(FROM, TO)
}

/// Swaps Bevy's shadows.wesl for the faded one; after PbrPlugin, which embeds it. Without PBR there's nothing to fade.
pub fn install(app: &mut App) {
    if !raea_sys_frame::bevy_api::swap_embedded_shader(app, "bevy_pbr/render/shadows.wesl", UPSTREAM, faded) {
        debug!("raea-sys-lod: no bevy_pbr shadows.wesl to fade");
        return;
    }
    // PbrPlugin started loading Bevy's file when it was built, and an embedded file is read on the IO threads at once,
    // so by the time this plugin is built it already holds Bevy's bytes (on 11-17 the fly drew bit for bit as before
    // the swap): read again, from the swapped file
    if let Some(server) = app.world().get_resource::<AssetServer>() {
        server.reload("embedded://bevy_pbr/render/shadows.wesl");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fade replaces the end of the cascade blend once and adds the last cascade's branch.
    #[test]
    fn the_last_cascade_fades() {
        let bevy = format!("fn fetch_directional_shadow() -> f32 {{\n    if (next_cascade_index < n) {{\n        if (x) {{\n{FROM}\n");
        let ours = faded(&bevy);
        assert_eq!(ours.matches("raea: the last cascade fades").count(), 1);
        assert!(ours.contains("shadow = mix(shadow, 1.0,"));
    }
}
