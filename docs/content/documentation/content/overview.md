+++
title = "Overview"
weight = 10
+++

Ansorum's primary content unit is an answer page: one Markdown file per
answerable unit with first-class answer metadata. The inherited section/page
model still exists underneath because the repository reuses much of Zola's
compiler, but the authoring contract should now be understood through the
answer-first lens.

A typical answer corpus looks like this:

```bash
.
└── content
    ├── refunds.md
    ├── refunds.schema.json
    ├── cancel.md
    └── internal-playbook.md
```

Each `.md` file becomes a page route for humans and may also emit machine
artifacts such as `/page.md` and `schema.json`. The output path can still be
customized through page frontmatter such as `path` or `slug`, but the critical
contract is the answer metadata documented on the
[page frontmatter page](@/documentation/content/page.md#ansorum-answer-front-matter).
Structured-data sidecars use the matching file stem, for example
`refunds.md` with `refunds.schema.json`.

## What Belongs In Content

Put these things in `content/`:

- authored answer Markdown files
- optional co-located assets referenced by those answers
- optional `<answer-stem>.schema.json` sidecars for JSON-LD
- optional `_index.md` files when you need section metadata or landing pages

Keep authored answer Markdown focused on one canonical answer, not broad mixed
topic pages. Use frontmatter to declare intent, audience, visibility, and
canonical questions explicitly.

## Asset colocation

The `content` directory is not limited to Markdown files. It is natural to
co-locate an answer and some related assets, such as images, PDFs, or schema
files. Ansorum supports this pattern out of the box.

All non-Markdown files in a page or section directory are copied alongside the
generated page when the site is built, which allows relative links to work.

Pages with co-located assets should usually live in a dedicated directory with
`index.md`, like this:


```bash
└── content
    └── refunds
        ├── index.md
        ├── refunds.schema.json
        └── refund-window.png
```

With that setup, you can link to `refund-window.png` directly from the Markdown
and keep the schema sidecar next to the answer source.

```Markdown
See the chart [here](refund-window.png).
```

By default, this page's slug will be the directory name and its permalink will
match that path.

### Excluding files from assets

It is possible to ignore selected asset files using the
[ignored_content](@/documentation/getting-started/configuration.md) setting in the config file.
For example, say that you have several code files which you are linking to on your website.
For maintainability, you want to keep your code in the same directory as the Markdown file,
but you don't want to copy the build folders to the public web site. You can achieve this by setting `ignored_content` in the config file:

(Note of caution: `{Cargo.lock,target}` is _not_ the same as `{Cargo.lock, target}`)
```
ignored_content = ["code_articles/**/{Cargo.lock,target}, *.rs"]
```

## Static assets

In addition to placing content files in the `content` directory, you may also place content
files in the `static` directory.  Any files/directories that you place in the `static` directory
will be copied, without modification, to the `public` directory.

Typically, you might put site-wide assets (such as a CSS file, the site favicon, site logos or site-wide
JavaScript) in the root of the static directory. You can also place any HTML or other files that
you wish to be included without modification (that is, without being parsed as Markdown files)
into the static directory.

Note that the static directory provides an _alternative_ to co-location.  For example, imagine that you
had the following directory structure (a simplified version of the structure presented above):

```bash
.
└── content
    └── blog
        ├── configuration
        │    └── index.md // -> https://mywebsite.com/blog/configuration/
        └── _index.md // -> https://mywebsite.com/blog/
```

To add an image to the `https://mywebsite.com/blog/configuration` page, you have three options:
 *  You could save the image to the `content/blog/configuration` directory and then link to it with a
 relative path from the `index.md` page.  This is the approach described under **co-location**
 above.
 *  You could save the image to a `static/blog/configuration` directory and link to it in exactly the
 same way as if you had co-located it. If you do this, the generated files will be identical to those
 obtained if you had co-located the image; the only difference will be that all static files will be saved in the
 static directory rather than in the content directory. The choice depends on your organizational needs.
 *  Or you could save the image to some arbitrary directory within the static directory. For example,
 you could save all images to `static/images`.  Using this approach, you can no longer use relative links. Instead,
 you must use an absolute link to `images/[filename]` to access your
 image. This might be preferable for small sites or for sites that associate images with
 multiple pages (e.g., logo images that appear on every page).
