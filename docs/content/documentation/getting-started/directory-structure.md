+++
title = "Directory structure"
weight = 30
+++

After running `ansorum init`, you should see the following structure:


```bash
.
├── config.toml
├── content
├── sass
├── static
├── templates
└── themes

5 directories, 1 file
```

You might also see a `public` directory after running `ansorum build` or
`ansorum serve`. In an Ansorum project, `public` is where compiled human and
machine outputs land.

The current scaffold is intentionally minimal. The answer-first shape appears
when you add authored answers, sidecars, packs, and eval fixtures. The
reference project looks more like this:

```bash
.
├── collections/
│   └── packs/
├── config.toml
├── content/
│   ├── refunds.md
│   ├── refunds.schema.json
│   ├── cancel.md
│   └── internal-playbook.md
├── eval/
│   └── fixtures.yaml
└── public/
```

Here's the role of each directory and `config.toml`.

## `config.toml`
A mandatory Zola configuration file in TOML format.
This file is explained in detail in the [configuration documentation](@/documentation/getting-started/configuration.md).

## `content`
Contains authored answer Markdown and any co-located assets. For Ansorum, each
answerable unit should usually be one `.md` file with first-class answer
frontmatter. Sidecar structured data lives beside that file as
`<answer>.schema.json`.

To learn more, read the [content overview page](@/documentation/content/overview.md).

## `sass`
Contains the [Sass](https://sass-lang.com) files to be compiled. Non-Sass files will be ignored.
The directory structure of the `sass` folder will be preserved when copying over the compiled files; for example, a file at
`sass/something/site.scss` will be compiled to `public/something/site.css`.

## `static`
Contains any kind of file. All the files/directories in the `static` directory will be copied as-is to the output directory.
If your static files are large, you can configure Zola to [hard link](https://en.wikipedia.org/wiki/Hard_link) them
instead of copying them by setting `hard_link_static = true` in the config file.

## `templates`
Contains the [Tera](https://keats.github.io/tera) templates used to render human
HTML pages. Ansorum's machine outputs such as `/page.md`, `answers.json`, and
`llms.txt` are compiler outputs, not hand-authored templates.

## `themes`
Contains themes that can be used for your site. If you are not planning to use themes, leave this directory empty.
If you want to learn about themes, see the [themes documentation](@/documentation/themes/_index.md).
