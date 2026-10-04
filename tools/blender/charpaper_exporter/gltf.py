import bpy

# Settings CharPaper depends on. The rest come from the panel's glTF settings.
LOCKED = {
    "export_yup": True,
    "export_rest_position_armature": True,
    "export_morph": True,
    "export_skins": True,
    "export_draco_mesh_compression_enable": False,
    "export_meshopt_compression_enable": False,
    "use_selection": False,
    "use_visible": False,
    "use_renderable": False,
    "use_active_collection": False,
    "use_active_scene": True,
    "export_extras": False,
    "export_leaf_bone": False,
    "export_hierarchy_flatten_bones": False,
    "export_armature_object_remove": False,
    "export_gpu_instances": False,
    "export_pointer_animation": False,
    "export_anim_single_armature": False,
    "export_reset_pose_bones": True,
    "export_bake_animation": False,
    "export_current_frame": False,
    "export_frame_range": False,
    "export_anim_slide_to_zero": True,
    "export_merge_animation": "NLA_TRACK",
    "will_save_settings": False,
}

EXTENSIONS = {"GLB": ".glb", "GLTF_SEPARATE": ".gltf"}


def available():
    return hasattr(bpy.ops.export_scene, "gltf")


def settings_options(gltf):
    return {
        "export_format": gltf.file_format,
        "export_image_format": gltf.image_format,
        "export_apply": gltf.apply_modifiers,
        "export_tangents": gltf.tangents,
        "export_vertex_color": gltf.vertex_color,
        "export_import_convert_lighting_mode": gltf.lighting_mode,
        "export_def_bones": gltf.deform_bones_only,
        "export_force_sampling": gltf.force_sampling,
        "export_frame_step": gltf.frame_step,
        "export_optimize_animation_size": gltf.optimize_animation,
    }


def export(filepath, options):
    """Exports the window's current scene. Returns the options this exporter lacks."""
    known = {p.identifier: p for p in bpy.ops.export_scene.gltf.get_rna_type().properties}
    kwargs, skipped = {}, []
    for key, value in options.items():
        prop = known.get(key)
        if prop is None:
            skipped.append(key)
        elif (
            prop.type == "ENUM"
            # Enums whose items come from a function report none here, so they can't be checked.
            and len(prop.enum_items) > 0
            and value not in {item.identifier for item in prop.enum_items}
        ):
            skipped.append(f"{key}={value}")
        else:
            kwargs[key] = value
    bpy.ops.export_scene.gltf(filepath=filepath, **kwargs)
    return skipped
