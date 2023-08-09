"use client";
import Search from "./search";
import { SearchProvider } from "retake-search";

const Page = () => {
  return (
    <SearchProvider apiKey={"retake-test-key"} url={"http://localhost:8000"}>
      <div className="w-full h-full p-12">
        <Search />
      </div>
    </SearchProvider>
  );
};

export default Page;
