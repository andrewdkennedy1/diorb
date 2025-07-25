# DIORB Navigation Test Guide

## Fixed Issues
- ✅ Arrow keys no longer move selection twice
- ✅ Added vim-style navigation (j/k for up/down)
- ✅ Consistent key handling across screens
- ✅ Proper quit handling (Esc to quit from start screen)

## Test Steps

### 1. Start Screen Navigation
1. Run `cargo run`
2. Use ↑/↓ or j/k to navigate between disks
3. Each key press should move selection by exactly one item
4. Selection should wrap around (top to bottom, bottom to top)
5. Press 'C' to open configuration
6. Press 'Esc' to quit

### 2. Configuration Screen Navigation
1. From start screen, press 'C' to open config
2. Use ↑/↓ or j/k to navigate between config fields
3. Press Enter/Space to open dropdown for selected field
4. In dropdown: use ↑/↓ or j/k to navigate options
5. Press Enter/Space to select option
6. Press 'S' to start test with current config
7. Press 'Esc' to go back to start screen

### 3. Test Flow
1. Select a disk from start screen
2. Press Enter to start 1GB speed test directly
   OR
   Press 'C' to configure, then 'S' to start test
3. Watch progress in running screen
4. View results when complete
5. Navigate results screen with ←/→ arrows
6. Press Enter to save or go back

## Expected Behavior
- Single key press = single movement
- Consistent navigation across all screens
- Clear visual feedback for selections
- Proper screen transitions
- No double-movement or skipped selections

## Controls Summary
- **↑↓ or jk**: Navigate up/down
- **←→ or hl**: Navigate left/right (results screen)
- **Enter/Space**: Select/Confirm
- **C**: Configuration (start screen)
- **S**: Start Test (config screen)
- **Esc**: Back/Quit
- **Q**: Quit (global, but handled per screen now)