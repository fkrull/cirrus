TARGET_DIR := $(CURDIR)/../src/resources
SIZES := 16 24 32 48

meta/all: \
		$(TARGET_DIR)/cirrus-idle.light.ico \
		$(TARGET_DIR)/cirrus-idle.dark.ico \

meta/clean:
	rm -r $(TARGET_DIR)/*

$(TARGET_DIR)/%.ico: $(foreach size,$(SIZES),$(TARGET_DIR)/$(size)/%.png)
	convert $^ $@

$(TARGET_DIR)/%/cirrus-idle.light.png: icon.svg
	bash export-merged-png-icon.sh "$<" "$@" "$*" "light"

$(TARGET_DIR)/%/cirrus-idle.dark.png: icon.svg
	bash export-merged-png-icon.sh "$<" "$@" "$*" "dark"

$(TARGET_DIR)/%/cirrus-running.light.png: icon.svg
	bash export-merged-png-icon.sh "$<" "$@" "$*" "light running"

$(TARGET_DIR)/%/cirrus-running.dark.png: icon.svg
	bash export-merged-png-icon.sh "$<" "$@" "$*" "dark running"

.PHONY: meta/*

.SECONDARY:
