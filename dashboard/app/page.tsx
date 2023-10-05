import { redirect } from "next/navigation";

const Index = async () => {
  redirect("/dashboard");
};

export default Index;
