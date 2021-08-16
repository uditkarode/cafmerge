# cafmerge
A blazing-fast utility that lets you easily merge CAF tags into your ROM source.

# Usage
First, you need a custom manifest:
BEFORE:
```xml
...
<remove-project name="platform/art" />
<project path="art" name="android_art" remote="404" />
...
```

AFTER:
```xml
...
<remove-project name="platform/art" />
<project path="art" name="android_art" remote="404" caf="platform/art" />
...
```

You must do this for every entry that you want to merge newer tags into!

Now, you can use cafmerge as:
```bash
cafmerge --manifest /path/to/manifest.xml --tag LA.UM.KEK
```
  
This will begin merging the provided tag into the repos with the `caf` attribute in the manifest.
  
You can use `cafmerge --manifest /path/to/manifest.xml --show-conflicts` to list all repos among the ones in the manifest with conflicts for manual resolution.
