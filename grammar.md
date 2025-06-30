# grammar

```
... = [^\n]+
LINK = [^\s]+
LINK_TITLE = [^\n]+
FN = \d+

(^\n | \n\n)... => <p>

...\n\n => </p>

_..._ => <i>...</i>

*...* => <strong>...</strong>

#...\n+ => <h1>...\n

##...\n+ => <h2>...\n

[LINK] => <a href="LINK" target="_blank">LINK</a>

[LINK](LINK_TITLE) => <a href="LINK" target="_blank">LINK_TITLE</a>

[^FN] => <a id="link-FN" target="#ref-FN"><sup>FN</sup></a>

\n+[^FN]: ...(\n | EOF) => <p class="footnote" id="ref-{FN}"><span class="footnote">{FN}:</span> ...</p>

```
