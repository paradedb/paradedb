"use client";

const Card = () => {
  return (
    <div className="space-y-4 divide-y rounded animate-pulse divide-neutral-700">
      <div className="flex items-center justify-between">
        <div>
          <div className="h-2.5 rounded-full bg-neutral-600 w-24 mb-2.5"></div>
          <div className="w-32 h-2 rounded-full bg-neutral-700"></div>
        </div>
        <div className="h-2.5 rounded-full w-12"></div>
      </div>
      {[...Array(3)].map((_, i) => (
        <div className="flex items-center justify-between pt-4" key={i}>
          <div>
            <div className="h-2.5 rounded-full bg-neutral-600 w-24 mb-2.5"></div>
            <div className="w-32 h-2 rounded-full bg-neutral-700"></div>
          </div>
          <div className="h-2.5 rounded-full bg-neutral-700 w-12"></div>
        </div>
      ))}
    </div>
  );
};

export { Card };
