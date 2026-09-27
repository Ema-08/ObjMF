# Obj Models Formatter

Obj Models Formatter is a tool thought to format properly OBJ 3d models in order to make them work with Minecraft Forge's default OBJ loader.

## Tool's Configuration

This tool requires a compulsory configuration file to be present besides the executable, and named `objmf_config.json` (see configs.rs for a sketch of the expected format's scheme and the actual validation code).

The configuration file's content is a JSON object with the following required fields (note that other fields will be treated as an error):
 - `mod_id`: the mod's modid as specified in the mod's source code. This field must be a string matching `^[a-z]+$`.
 - `resources_path`: a path pointing to the project's resources folder (such as `src/main/resources` in gradle, usually). This field must be a string containing a valid path (links and other shortcuts such as `/../`) are allowed, though them will be parsed directly by the machine that is hosting the program and may thus be subsceptible to os-specific quirks.
 - `obj_rootf`: the path where to place formatted `.obj` files within the namespaced assets folder inside the mod's resources folder. This field must be a string matching `^[a-z]+(?:/[a-z]+)*$`.
 - `mtl_rootf`: the path where to place formatted `.mtl` files within the namespaced assets folder inside the mod's resources folder. This field must be a string matching `^[a-z]+(?:/[a-z]+)*$`.
 - `png_rootf`: the path where to place formatted `.png` files within the `textures` folder of the namespaced assets folder inside the mod's resources folder. This field must be a string matching `^[a-z]+(?:/[a-z]+)*$`.
 - `model_types`: an object mapping model type identifiers to paths used relatively to `*_rootf` paths. Its keys must be strings matching `^[A-Z](?:[A-Z_]*[A-Z])?$`, and its values must be strings matching `^[a-z]+(?:/[a-z]+)*$`. Each key represents a model type identifier, while its corresponding value is the path used for that model type's assets. At least one model type is required for the tool to work.

## Input restraints

This tool poses some restraints on the models it can handle (not necessarily due to technical limitations, but simply because I currently not need to do otherwise):
 - `.obj` files must declare exactly one `mtllib` to be used;
 - `.mtl` files must declare exactly one `map_Kd` to be used;
 - `.mtl` files must not declare any other `map` kind directives, such as `map_Ka` or `map_Ks`.

There are also some restraints on the model's name that is specified during formatting, as it must match `^[a-z](?:[a-z_]*[a-z])?$` pattern.

## Tool's workflow and output

Once the tool is properly configured, it can be opened and used by providing an OBJ model file, the corresponding material library and texture, compliant with the stated input restraints.

### Operations carried out on the model's texture file

This tool verifies the texture file header to match the standard `.png` header, but will not attempt to parse the full image to verify it is not malformed. No modifications are carried out on the texture file.

### Operations carried out on the model's material library file

Assuming an `.mtl` file compliant with the stated input restraints is provided, this tool will modify the material library's `map_Kd` to the following value: `<mod_id>:<png_rootf>/<model_type_path>/<model_name>`, in which `<mod_id>` and `<png_rootf>` are the values as specified in the tool's configuration, `<model_type_path>` is the value of the user-provided model type for the model (again, obtained from the configuration) and `<model_name>` is the user-provided model name.

This tool does not verify the overall integrity of the material library file and compliance with `.mtl` file format.

### Operations carried out on the model's file

Assuming an `.obj` file compliant with the stated input restraints is provided, the only modification this tool executes is renaming the `mtllib` directive in order to point to a material library file named with the user-provided model name. For instance, if the file contained the declaration as `mtllib lol.mtl`, the output will contain `mtllib <model_name>.mtl` instead, where `<model_name>` is the user-provided model name.

This tool does not verify the overall integrity of the model file and compliance with `.obj` file format.

### Output locations

The triplet of formatted files for a model are directly saved in the proper location within the mod project's resources folder, at the following locations:
 - Model file (`.obj`): `<resources_path>/assets/<mod_id>/<obj_rootf>/<model_type_path>/<model_name>.obj`;
 - Material Library file (`.mtl`): `<resources_path>/assets/<mod_id>/<mtl_rootf>/<model_type_path>/<model_name>.mtl`;
 - Texture file (`.png`): `<resources_path>/assets/<mod_id>/textures/<png_rootf>/<model_type_path>/<model_name>.png`.
(Each `<..>` segment of the paths is replaced with the proper variable. Names match ones declared for configuration or the already stated user-provided inputs.)

Note that the tool actively tries to reduce to bare minimum cases in which formatted files are only partially written to the resources folder, in order to prevent the project to be left in an inconsistent state. Moreover, the tool will refuse to overwrite files that already exist.

## Notes on maintenance

It is unlikely that I will constantly maintain this tool, unless changes are required to adapt it for projects I'm actively working on, though anyone should feel free to fork this repository to adapt the tool to their own needs, or just use it as-is.

## Legal

See `LICENSE.txt`.