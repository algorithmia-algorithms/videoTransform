# Video Transform
<img src="http://i.imgur.com/oziWGjx.png" width=800></img>

This algorithm uses an image processing algorithm to alter each frame of a video, then recombines it back into a freshly transformed video file.

**note:** This algorithm takes longer than most other algorithms as it recursively calls an image algorithm over each frame of a video file, make sure to add a custom timeout to your algorithm call.

**note:** This algorithm is royalty free, but the image processing algorithms it can call might not be, check your account's credit balance before running this algorithm on a custom video.


# Changelog
0.5.0 - May 23, 2017:
* Added [Smart Video Downloader][smd] support to resolve a compressed gzip request problem
* with Smart Video Downloader added, `input_file` may now point to any web hosted URL, including youtube, vimeo, etc.



# Table of Contents

*   [Input](#inputs)
*   [Output](#outputs)
*   [Default Options](#defaults)
*   [Examples](#examples)
*   [credits](#credits)
*   [Algorithm Console](#console)

<a id="inputs"></a>

# Input

```
input_file: String,
output_file: String,
algorithm: String,
advanced_input: Json,
fps: Double,
image_compression: Int,
video_compression: Int
```

*   input_file - **_(required)_** - The input url of the video file, can be any standard video container format (mp4, mkv, avi, etc), can be an http/https url or a data connector uri(`data://`, `s3://`, `dropbox://`, etc).
*   output_file - **_(required)_** - The output url of the altered video file, must be a data connector uri (`data://`, `s3://`, `dropbox://`, etc).
*   algorithm - **_(required)_** - The image processing algorithmia algorithm uri, if no default mode is found for that algorithm, advanced_input _must_ be defined.
*   advanced_input - **_(optional)_** - if you have advanced requirements or want to use an algorithm with no default parameters, See [Advanced Input](#advancedInput).
*   fps - **_(optional)_** - If you have a desired fps sampling rate, input it here. _defaults to the input video's fps._
*   image_compression - **_(optional)_** - if you want to improve performance of processing, it's possible to compress each frame using the jpeg compression algorithm, the value provided is the associated compression ratio.
*   video_compression - **_(optional)_** - by default, the output video file is raw and uncompressed, if you desire to compress your output video using the libx264 codec, provide a compression ratio value for this element.

<a id="advancedInput"></a>

## Advanced Input

If advanced_input is found, this algorithm will parse it as a json object; finding and replacing keywords relating to uploading and downloading files in both batch and single form with paths that the Video Transform algorithm uses.

The keywords are:

`$SINGLE_INPUT` - whenever you see a single image input path for an algorithm, this keyword will work

`$SINGLE_OUTPUT` whenever you see a single image output path for an algorithm, this keyword will work

`$BATCH_INPUT` - similar to single input keyword, but works for arrays of input files (DeepFilter for example)

`$BATCH_OUTPUT` - similar to single input keyword, but works for arrays of input files (DeepFilter for example)

## Example:

#### SalNet algorithm default input

```
{
  "image": "data://deeplearning/example_data/mona_lisa.jpg",
  "location": "data://.algo/temp/test42.png"
}

```

#### Video Transform with advanced salnet input

```

{  
   "input_file": "data://path/to/file.mp4",
   "output_file": "data://save/file.mp4,
   "algorithm": "algo://deeplearning/SalNet/0.1.6",
   "advanced_input": {
  "image": "$SINGLE_INPUT",
  "location": "$SINGLE_OUTPUT"
},
   "fps": Double
}

```

<a id="outputs"></a>

# Output

```
{  
    "output_file": String
}

```

*   output_file - the complete algorithmia URI for where the output video file has been uploaded to.

<a id="defaults"></a>

# Default Options

This algorithm has default parameters set up for a number of algorithms, this list will grow as new image altering algorithms are released:

*   **DeepFilter** - uses version `0.6.0`, `filterName` is `gan_vogh` - (deeplearning/DeepFilter)
*   **SalNet** - uses version `0.2.0` - (deeplearning/SalNet)
*   **Colorful Image Colorization** - uses version `1.1.6` - (deeplearning/ColorfulImageColorization)

<a id="examples"></a>

# Examples

### DeepFilter default input

```
{  
   "input_file":"data://media/videos/lounge_demo.mp4",
   "output_file":"data://.algo/temp/altered_lounge_demo.mp4",
   "algorithm":"algo://deeplearning/DeepFilter"
}

```

### DeepFilter advanced input

```
{  
   "input_file":"data://media/videos/lounge_demo.mp4",
   "output_file":"data://.algo/temp/altered_lounge_demo.mp4",
   "algorithm":"algo://deeplearning/DeepFilter/0.3.2",
   "advanced_input":{  
      "images":"$BATCH_INPUT",
      "savePaths":"$BATCH_OUTPUT",
      "filterName":"neo_instinct"
   }
}

```

### SalNet default input with fps

```
{  
   "input_file":"data://media/videos/lounge_demo.mp4",
   "output_file":"data://.algo/temp/altered_lounge_demo.mp4",
   "algorithm":"algo://deeplearning/SalNet",
   "fps": 14.24
}

```

<a id="credits"></a>

# Credits

All video processing is handled by `FFMpeg`, which is a fantastic video/image manipulation tool that can be found [here](https://ffmpeg.org/)<a id="console"></a>

[smd]: https://algorithmia.com/algorithms/media/SmartVideoDownloader