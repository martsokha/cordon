# Model conversion pipeline.
#
# Converts every .fbx and .dae file under raw/models/ into a .glb
# file under assets/models/ using assimp. Only reconverts files
# whose source is newer than the existing .glb.
#
# Prerequisites:
#   brew install assimp
#
# Usage:
#   make models     — convert all changed models
#   make clean      — delete all generated .glb files

RAW_DIR   := raw/models
OUT_DIR   := assets/models

FBX_SRC   := $(shell find $(RAW_DIR) -name '*.fbx' 2>/dev/null)
DAE_SRC   := $(shell find $(RAW_DIR) -name '*.dae' 2>/dev/null)

FBX_GLB   := $(FBX_SRC:$(RAW_DIR)/%.fbx=$(OUT_DIR)/%.glb)
DAE_GLB   := $(DAE_SRC:$(RAW_DIR)/%.dae=$(OUT_DIR)/%.glb)

ALL_GLB   := $(FBX_GLB) $(DAE_GLB)

.PHONY: models clean

models: $(ALL_GLB)
	@echo "$(words $(ALL_GLB)) models up to date."

$(OUT_DIR)/%.glb: $(RAW_DIR)/%.fbx
	@mkdir -p $(dir $@)
	assimp export $< $@ -f glb2
	@echo "  $< → $@"

$(OUT_DIR)/%.glb: $(RAW_DIR)/%.dae
	@mkdir -p $(dir $@)
	assimp export $< $@ -f glb2
	@echo "  $< → $@"

clean:
	rm -f $(ALL_GLB)
	@echo "Cleaned generated .glb files."
