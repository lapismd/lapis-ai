import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
export async function verifyExisting(
  name,
  version,
  request = fetch,
  read = readFileSync,
) {
  const url = `https://crates.io/api/v1/crates/${name}/${version}`;
  const options = {
    headers: {
      "User-Agent":
        "lapis-ai release verification (github.com/lapismd/lapis-ai)",
    },
  };
  const metadata = await request(url, options);
  if (metadata.status === 404) return false;
  if (!metadata.ok)
    throw Error(`registry metadata ${name}: HTTP ${metadata.status}`);
  const response = await request(`${url}/download`, options);
  if (!response.ok)
    throw Error(`registry download ${name}: HTTP ${response.status}`);
  const data = Buffer.from(await response.arrayBuffer());
  const local = read(`target/package/${name}-${version}.crate`);
  const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
  if (digest(data) !== digest(local))
    throw Error(
      `${name} already exists with different package contents; stop publication`,
    );
  return true;
}
