const fs = require("fs");
const path = require("path");
const matter = require("matter");
const parse = require("remark-parse");

const { Client } = require("retake-search");

const directoryPath = "../../docs";
const indexName = "docs";
const client = new Client("retake-test-key", "http://localhost:8000");

const HEADING = "heading";

const urlFriendlyString = (title) => {
  return title
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9\-]/g, "");
};

const extractSectionsFromASTNode = (node, fileName, title) => {
  const sections = [];
  let currentSection = { title, content: "", path: fileName };

  import("remark").then((remark) => {
    for (let child of node.children) {
      if (child.type === HEADING) {
        if (currentSection.title || currentSection.content.trim()) {
          sections.push(currentSection);
        }

        const title = child.children[0].value;
        currentSection = {
          title: title,
          content: "",
          path: `${fileName}#${urlFriendlyString(title)}`,
        };
      } else {
        const serializedContent = remark().stringify(child);
        currentSection.content += serializedContent + "\n\n";
      }
    }
  });

  if (currentSection.title || currentSection.content.trim()) {
    sections.push(currentSection);
  }

  return sections;
};

const parseMdx = async (fileName, mdxString) => {
  const {
    data: { title },
    content,
  } = matter(mdxString);

  await import("remark").then((remark) => {
    import("remark-parse").then(async (parse) => {
      const ast = await remark().use(parse).parse(content);
    });
  });
  const sections = extractSectionsFromASTNode(ast, fileName, title);

  return sections;
};

const readMdxFiles = (directoryPath) => {
  return new Promise((resolve, reject) => {
    fs.readdir(directoryPath, (err, files) => {
      if (err) {
        reject(`Error reading directory: ${err}`);
        return;
      }

      const mdxFiles = files.filter((file) => path.extname(file) === ".mdx");

      const filesContent = mdxFiles.map((file) => {
        const filePath = path.join(directoryPath, file);
        const content = fs.readFileSync(filePath, "utf-8");
        return { name: file, content };
      });

      resolve(filesContent);
    });
  });
};

const setup = async () => {
  const files = await readMdxFiles(directoryPath);

  let index;
  try {
    index = await client.getIndex(indexName);
  } catch (err) {
    index = await client.createIndex(indexName);
  }

  console.log("Indexing files...");

  await Promise.all(
    files.map(async (file) => {
      const fileName = file.name.replace(".mdx", "");
      const parsed = await parseMdx(fileName, file.content);
      const documents = parsed.map((doc) => ({
        content: doc.content ?? "",
        title: doc.title ?? "",
      }));
      const ids = parsed.map((doc) => doc.path);
      await index.upsert(documents, ids);
    })
  );

  console.log("Vectorizing fields...");
  await index.vectorize(["title", "content"]);
  console.log("Vectorizing successful!");
};

setup();
