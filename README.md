# ios2exif

Simple command line utility to rename all images in the current working directory to the EXIF `DateTimeOriginal` attribute. For example, this would rename `IMG_0975.JPG` to `2023-05-14_21-08-53.JPG`. Takes into account files with identical timestamps, throwing an error if that's the case.
