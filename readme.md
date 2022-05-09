# staticcc

**static** **c**ontent **c**reator

staticcc is like drneo, but built for local first, rather than local as an afterthought.
the control flow for drneo's local mode is horrible, so i decided to redo the whole thing.
there's also some differences in replacements and templating, explained below

for a production example, check out [my website](https://ambylastname.xyz) and its [source code](https://github.com/ambyshframber/wobsite_v3)

## basic usage

when run with no directory arguments, staticcc expects a directory with the following structure:

- root
    - cfg *
        - templates *
            - main
            - (other template files here)
        - md_ignore
        - md_replace
    - site *
        - (site content here)

files and directories marked with `*` are required. additionally, a `build` directory will be created relative to the root, or deleted and recreated if it already exists.

`md_ignore` should contain a list of paths (relative to `site`) that will be sent to the build directory without processing.
`md_replace` should contain a list of replacements in multiline SCF ("**s**taticcc **c**onfig **f**ormat"). more on that later.

`templates` is more complicated. a full explanation of the template format and templating system will follow.

## SCF

SCF stands for "**s**taticcc **c**onfig **f**ormat", if you're in a good mood, or "shit config format"/"stupid config format" if you aren't.
it comes in 2 major variants: single-line and multiline.
single-line SCF goes like this:
```
KEY1=VALUE1
KEY2=VALUE2
```
each key-value pair is its own line. line breaks cannot be used. `=` can be used in values but not keys.

multiline SCF goes like this:
```
KEY1
value 1
can be multiple lines
----
KEY2
value 2
----
KEY3=VALUE3
```
multiline SCF blocks are delimited by `----` on a line on its own. the first line of a block is the key,
and the remainder are the value, UNLESS the block is only one line. if this happens, the block is parsed as single-line SCF.

SCF is used for replacements and front matter information.

## replacements

the `md_replace` file should contain a list of replacements in multiline SCF.
when a markdown file is being processed, keys will be replaced with their values in a slightly convoluted way.
the key will be prepended with `REP=`, and then every unescaped (ie. not preceeded by a backslash) instance of that will be replaced with the value.
for example, if we had the key `beans` and the value `lorem ipsum`,
every instance of `REP=beans` (but not `\REP=beans`) in your documents would be replaced with `lorem ipsum`.

replacements can also be passed in via the command line, in single-line SCF.

## document sections

document sections are delimited with `##(NAME)##`. specifically, they are delimited by any substring that matches `(^|[^\\])##([^#\n]+)##`,
which is to say "anything that's not a backslash, followed by 2 hashes,
followed by any number of characters that are not a hash or newline, followed by another 2 hashes".
these are important for templating.

## front matter

staticcc uses front matter for per-file config. it goes like this:
```
---
KEY=VALUE
KEY2=VAL2
---

(remainder of document here)
```
it's scf again. sorry.

## templating

the `cfg/templates` directory is where template files are kept. templates are specified on files using the front matter:
```
---
template=blog
---
##BODY##
foo bar
```
the above example will use the template at `cfg/templates/blog`. if no template is given, `cfg/templates/main` is used.

templates take the following format:
```html
<html>
<head>
<title>##TITLE##</title>
</head>
<body>
<div class="main">
##BODY##
</div>
<div class="footer">
##FOOT##
</div>
</body>
</html>
```
every document section is substituted into the template according to it's name. for example, if you have
```md
##BODY##

this is the body text!

it can be multiple lines

##FOOT##

beans
```
the section starting with `##BODY##` will go into the div with class "main",
and the section starting with `##FOOT##` will go into the div with class "footer".

after sections have been substituted in, data is taken from the front matter.
for example, if you had `TITLE=homepage` in the markdown file's front matter, "homepage" would be placed inside the title tag.
this would work with any key/value pair.

after sections and front matter are done, unused tags are cleaned up with some regex magic.

## blogging

staticcc has support for multiple concurrent rss channels, all of which are configured in the `cfg/channels` file.
the file is multiline SCF, where the key of each block is the internal channel id, and the value is some single-line SCF.

an example config would be:
```
feed1
title=example blog!
description=a demo blog for the staticcc program
page=blog
image=rss_image.png
outfile=blog/rss.xml
prepend=https://example.com/
----
(another config block here)
```
this would create a channels with the id `feed1`.

channel keys are:
- `title`: the title of the rss feed *
- `path`: the blog page, from site root *
- `prepend`: a string to prepend to all paths (usually just your domain name) *
- `outfile`: the path to place the output rss xml, from site root *
- `description`: the feed description. this is required for the rss spec, but if it's left out here, staticcc uses the empty string
- `image`: the path to the image to use. link and title sub-tags are generated automatically
keys marked with `*` are required

pages are added to channels with some front matter keys.
- `rss_chan_id`: determines the relevant feed *
- `rss_title`: the item title (falls back on `title` which can be useful for preventing magic numbers) *
- `rss_description`: the item description. again, this is required for the spec, but staticcc will use the empty string
- `rss_pubdate`: the publication date of the item, in rfc2822 format. staticcc checks this for you and will refuse to publish the feed if the format is wrong
keys marked with `*` are required

check out [the spec](https://www.rssboard.org/rss-specification) for more info on rss

## command line arguments

directories can be changed from the command line with the `-d`, `-i`, `-o` and `-c` options.

- `-d` changes the working directory
- `-i` changes the input directory
- `-o` changes the output directory
- `-c` changes the config directory

if the working directory is changed, the i/o/c folders are located at `site`, `build` and `cfg`, respectively.
for example if you ran staticcc with `-d ../beans`, the input directory would be `../beans/site`.

an important thing to note is that if the input, output or config directories are changed at the same time as the working directory,
they must be either relative to the new working directory, or absolute from system root.

the next set of arguments are `-I` and `-R`, meaning "ignore" and "replace" respectively.  
"ignore" here doesn't mean "completely ignore this file", but more "treat this file as plaintext and output it verbatim".
this means you can have a markdown file on your site without it getting turned into html.  
`-R` takes values in single-line SCF (eg. `-R KEY=VALUE`), which are then treated like standard replacements.

the final set is to do with markdown specification control. these are almost identical to the drneo markdown controls.

- `-s`: enable strikethrough
- `-t`: enable tables
- `-a`: enable autolink
- `-l`: enable task lists
- `-S`: enable superscript
- `-f`: enable footnotes
- `-D`: enable description lists
