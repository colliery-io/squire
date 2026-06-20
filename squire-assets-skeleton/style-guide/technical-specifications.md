Technical Specifications

Version: 1.0

Purpose:

This document defines the technical requirements for all Squire visual assets.

These specifications ensure:

* Consistent exports
* Consistent naming
* Predictable implementation
* Easier asset management
* Future scalability

All visual assets must comply with these standards unless explicitly documented otherwise.

⸻

Asset Naming Convention

Format:

[type][category][name]

Examples:

icon_coin_star

icon_gem_ruby

icon_quest_scroll

badge_streak_master

banner_village_market

texture_parchment

⸻

Naming Rules

Use:

* lowercase
* underscores
* descriptive names

Avoid:

* spaces
* special characters
* version numbers in filenames

Good:

icon_reward_chest

Bad:

RewardChest_v2_FINAL.png

⸻

Asset Categories

Approved prefixes:

icon_

badge_

banner_

illustration_

texture_

decor_

avatar_

profile_

⸻

File Formats

Icons

Format:

PNG

Background:

Transparent

Compression:

Lossless

⸻

Badges

Format:

PNG

Background:

Transparent

Compression:

Lossless

⸻

Decorative Assets

Format:

PNG

Background:

Transparent

Compression:

Lossless

⸻

Illustrations

Format:

PNG

Background:

As required by composition

Compression:

Lossless

⸻

Textures

Format:

PNG

Compression:

Lossless

Requirements:

Tileable where specified

⸻

Master Asset Sizes

Master assets should always be generated at the highest approved resolution.

Derivative sizes can be produced later.

Never generate only a small version.

⸻

Icon Specifications

Master Size:

1024 × 1024

Format:

PNG

Background:

Transparent

Composition:

Single primary object

Occupancy:

70%–80% of canvas

⸻

Badge Specifications

Master Size:

1024 × 1024

Format:

PNG

Background:

Transparent

Occupancy:

70%–80%

⸻

Decorative Asset Specifications

Master Size:

1024 × 1024

Format:

PNG

Background:

Transparent

Examples:

* Shields
* Heraldry
* Corner decorations
* UI flourishes

⸻

Texture Specifications

Master Size:

512 × 512

Format:

PNG

Requirements:

Seamless tile

Repeat-safe

No visible seams

No baked borders

⸻

Banner Specifications

Master Size:

2048 × 512

Format:

PNG

Purpose:

* Headers
* Featured sections
* Promotional UI

Safe Area:

Keep critical content within center 75%.

Allow edge cropping.

⸻

Hero Illustration Specifications

Master Size:

2048 × 1152

Aspect Ratio:

16:9

Format:

PNG

Purpose:

* Onboarding
* Marketing
* Splash screens
* Empty states

Safe Area:

Central 70%.

Critical content should not touch edges.

⸻

Profile & Avatar Assets

Master Size:

1024 × 1024

Format:

PNG

Background:

Transparent

Must remain recognizable when displayed at:

64 × 64

and

128 × 128

⸻

Transparency Standards

Use transparency whenever possible.

Required:

* Icons
* Badges
* Decorative assets
* Avatars

Avoid baked backgrounds.

Backgrounds should be composable by the application.

⸻

Padding & Safe Margins

Icons:

Maintain approximately 10%–15% outer padding.

Badges:

Maintain approximately 10%–15% outer padding.

Illustrations:

Maintain composition-safe margins.

Avoid edge collisions.

⸻

Lighting Standard

All icons and object assets should follow:

Primary Light Source:

Upper Left

Secondary Fill:

Lower Right

Maintain consistency across the entire library.

⸻

Export Quality

Reject exports containing:

* Compression artifacts
* Blurry edges
* Cropping mistakes
* Transparency errors
* Generation artifacts
* Inconsistent resolutions

⸻

Color Management

Color Space:

sRGB

All exported assets should use sRGB.

Avoid:

* CMYK
* Wide-gamut exports
* Print-oriented profiles

The application is digital-first.

⸻

Asset Source Management

For every approved asset maintain:

Asset Name

Prompt Version

Creation Date

Review Status

Approved Version

Example:

Asset:
icon_coin_star

Prompt:
v3

Status:
Approved

Version:
1.0

⸻

Versioning

Asset versions should be tracked in documentation.

Do not include versions in filenames.

Use:

reviews/
accepted.md

reviews/
revisions.md

for version tracking.

⸻

Prompt Tracking

Every production asset should have a corresponding prompt file.

Example:

prompts/currency/icon_coin_star.md

The prompt file becomes the source of truth for regeneration.

⸻

Directory Structure

Recommended:

style-guide/

asset-catalog/

prompts/

generated-assets/

reviews/

roadmap/

All generated assets should be stored according to category.

⸻

Future-Proofing

When possible:

Generate larger assets.

Downscale later.

Never upscale production assets.

Master assets should be treated as archival-quality source material.

⸻

Technical Approval Checklist

Before approving an asset:

□ Correct filename

□ Correct category

□ Correct resolution

□ Correct format

□ Correct transparency

□ Correct color space

□ Correct lighting direction

□ Correct padding

□ No export artifacts

□ Matches technical requirements

If any item fails:

Revision Required.