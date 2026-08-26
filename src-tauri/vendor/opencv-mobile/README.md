# Embedded OpenCV dependency

Docsy statically embeds the minimal OpenCV 4.13.0 build from
[`nihui/opencv-mobile`](https://github.com/nihui/opencv-mobile), release `v36`.
It is used only by the local video-frame selection enhancer. End users do not
download or install OpenCV separately.

Included targets:

- macOS ARM64: the ARM64 slice of `opencv2.framework` as `libopencv2.a`.
- Windows x64 (VS2022): `opencv_core4130.lib` and `opencv_imgproc4130.lib`.
- Shared OpenCV C++ headers from the Windows x64 package.

Upstream archive verification:

- `opencv-mobile-4.13.0-macos.zip`:
  `sha256:5f510e607b0ff53c1a0b32e0c1dedad330b5e61ed5d7a81fe98c735df78badf8`
- `opencv-mobile-4.13.0-windows-vs2022.zip`:
  `sha256:e989b879e32d7b63cd1e6e810cc0b9e4eacbdd53081d7651acb359744e5e7b06`

OpenCV and opencv-mobile are distributed under the Apache License 2.0.
